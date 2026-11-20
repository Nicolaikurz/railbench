use std::collections::HashSet;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2 as EguiVec2};
use railbench_core::geometry::{
    norm_angle_deg, rotate, sample_arc_between_connectors, sample_flex_spline_points,
};
use railbench_core::types::{
    BaseboardConfig, Catalog, CatalogEntry, Layout, Model3D, TrackPiece, Vec2,
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

#[allow(clippy::too_many_arguments)]
pub fn draw_canvas_2d(
    ui: &mut egui::Ui,
    layout: &mut Layout,
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

    // 1. Millimeter Grid
    draw_grid(&painter, rect, camera, dark_mode);

    // 2. Baseboards & multi-table snap guides
    let board_header_rects = draw_baseboards(
        &painter,
        rect,
        camera,
        layout,
        catalogs,
        active_board_id.as_deref(),
        board_snap_guides,
        dark_mode,
    );

    let active_layer_id = layout
        .active_layer_id
        .as_deref()
        .unwrap_or("layer-1")
        .to_string();
    let layer_height_of = |lid: &str| -> f64 {
        layout
            .layers
            .as_ref()
            .and_then(|ls| ls.iter().find(|l| l.id == lid))
            .and_then(|l| l.height_mm)
            .unwrap_or(0.0)
    };

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
                catalogs,
                lid,
                is_active_layer,
                lcolor.as_deref(),
            );
        }

        // 2.5A. Ground terrain (lakes, rivers, meadows) renders BELOW tracks
        if let Some(models) = &layout.models {
            for m in models {
                let m_lid = m.layer_id.as_deref().unwrap_or("layer-1");
                if m_lid != lid.as_str() || m.builtin_id.as_deref() != Some("ground-terrain") {
                    continue;
                }
                let is_sel = selected.contains(&m.id);
                draw_model_2d(&painter, rect, camera, m, is_sel, is_active_layer, show_labels);
            }
        }

        // 3. Tracks on this layer (sorted by piece elevation so ramps/bridges draw over lower tracks)
        let mut layer_pieces: Vec<&TrackPiece> = layout
            .pieces
            .iter()
            .filter(|p| p.layer_id.as_deref().unwrap_or("layer-1") == lid.as_str())
            .collect();
        layer_pieces.sort_by(|a, b| {
            let ea = a.elevation.map(|e| (e.start + e.end) * 0.5).unwrap_or(0.0);
            let eb = b.elevation.map(|e| (e.start + e.end) * 0.5).unwrap_or(0.0);
            ea.partial_cmp(&eb).unwrap_or(std::cmp::Ordering::Equal)
        });
        for piece in layer_pieces {
            let is_sel = selected.contains(&piece.id);
            let entry = find_entry(catalogs, &piece.catalog_id);
            let piece_power = power_sim.and_then(|sim| sim.results.get(&piece.id));
            draw_track_piece(
                &painter,
                rect,
                camera,
                piece,
                &layout.pieces,
                entry,
                is_sel,
                is_active_layer,
                color_mode,
                lcolor.as_deref(),
                show_labels,
                power_sim_enabled,
                piece_power,
            );
        }

        // 3.5. Buildings, trees, signals, lamps & mountains render ABOVE tracks on this layer
        if let Some(models) = &layout.models {
            for m in models {
                let m_lid = m.layer_id.as_deref().unwrap_or("layer-1");
                if m_lid != lid.as_str() || m.builtin_id.as_deref() == Some("ground-terrain") {
                    continue;
                }
                let is_sel = selected.contains(&m.id);
                draw_model_2d(&painter, rect, camera, m, is_sel, is_active_layer, show_labels);
            }
        }
    }
    let _ = layer_height_of;

    // Short-circuit warning markers on top of tracks when Power Simulation is enabled
    if power_sim_enabled {
        if let Some(sim) = power_sim {
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
                let tag_r = Rect::from_center_size(tag_pos, EguiVec2::new(48.0, 15.0));
                painter.rect_filled(tag_r, 4.0, Color32::from_rgb(153, 27, 27));
                painter.text(tag_pos, egui::Align2::CENTER_CENTER, "SHORT!", FontId::proportional(9.5), Color32::WHITE);
            }
        }
    }

    // 4. Connectors (snap points) & Connector '+' Quick-Add buttons
    let mut plus_buttons: Vec<(String, usize, Pos2, bool)> = Vec::new();
    let models_slice: &[Model3D] = layout.models.as_deref().unwrap_or(&[]);
    let hover_pos = ui.input(|i| i.pointer.hover_pos());

    let mut hovered_open_conn: Option<(String, usize)> = None;
    if show_connectors && !terrain_brush.active && !measure_mode && dragging_catalog.is_none() {
        if let Some(hp) = hover_pos {
            let mut best_d = f32::INFINITY;
            for piece in &layout.pieces {
                if !is_layer_editable(layout, piece.layer_id.as_deref()) {
                    continue;
                }
                for (cidx, conn) in piece.connectors.iter().enumerate() {
                    if conn.connected_to.is_some()
                        || crate::building_snap::is_connector_connected_to_building(
                            conn.x, conn.y, models_slice, 6.0,
                        )
                    {
                        continue;
                    }
                    let conn_sp = camera.world_to_screen(rect, conn.x, conn.y);
                    let rad = conn.angle.to_radians();
                    let plus_sp = camera.world_to_screen(
                        rect,
                        conn.x + 28.0 * rad.cos(),
                        conn.y + 28.0 * rad.sin(),
                    );
                    let dc = hp.distance(conn_sp);
                    let dp = hp.distance(plus_sp);
                    let hit_r = (16.0 * camera.zoom.sqrt()).clamp(15.0, 24.0);
                    let d = dc.min(dp);
                    if d <= hit_r && d < best_d {
                        best_d = d;
                        hovered_open_conn = Some((piece.id.clone(), cidx));
                    }
                }
            }
        }
    }

    if show_connectors {
        for piece in &layout.pieces {
            if !is_layer_visible(layout, piece.layer_id.as_deref()) {
                continue;
            }
            let editable = is_layer_editable(layout, piece.layer_id.as_deref());
            let is_sel = selected.contains(&piece.id);
            for (cidx, conn) in piece.connectors.iter().enumerate() {
                let sp = camera.world_to_screen(rect, conn.x, conn.y);
                let rad = conn.angle.to_radians();
                let plus_sp = camera.world_to_screen(
                    rect,
                    conn.x + 28.0 * rad.cos(),
                    conn.y + 28.0 * rad.sin(),
                );
                let docked_to_bldg = crate::building_snap::is_connector_connected_to_building(
                    conn.x, conn.y, models_slice, 6.0,
                );
                if rect.expand(12.0).contains(sp) {
                    let r = (4.0 * camera.zoom).clamp(2.5, 6.5);
                    if conn.connected_to.is_some() || docked_to_bldg {
                        let fill = if docked_to_bldg {
                            Color32::from_rgb(2, 132, 199)
                        } else {
                            Color32::from_rgb(100, 116, 139)
                        };
                        painter.circle_filled(sp, r * 0.78, fill);
                    } else {
                        painter.circle_filled(sp, r, Color32::from_rgb(22, 163, 74));
                        painter.circle_stroke(sp, r + 1.5, Stroke::new(1.0_f32, Color32::from_rgb(34, 197, 94)));
                    }
                }

                let is_hovered_conn = hovered_open_conn
                    .as_ref()
                    .map_or(false, |(pid, ci)| pid == &piece.id && *ci == cidx);
                if editable && conn.connected_to.is_none() && !docked_to_bldg && (is_sel || is_hovered_conn) {
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

        // Building door connectors (engine-shed & roundhouse stalls)
        for m in models_slice {
            if !is_layer_visible(layout, m.layer_id.as_deref()) {
                continue;
            }
            for door in crate::building_snap::get_building_door_connectors(m) {
                let sp = camera.world_to_screen(rect, door.x, door.y);
                if !rect.expand(12.0).contains(sp) {
                    continue;
                }
                let docked = layout.pieces.iter().any(|p| {
                    p.connectors
                        .iter()
                        .any(|c| (c.x - door.x).hypot(c.y - door.y) <= 5.5)
                });
                let r = (4.5 * camera.zoom).clamp(3.0, 7.0);
                if docked {
                    painter.circle_filled(sp, r, Color32::from_rgb(2, 132, 199));
                    painter.circle_stroke(sp, r + 1.5, Stroke::new(1.2_f32, Color32::from_rgb(56, 189, 248)));
                } else {
                    painter.circle_filled(sp, r, Color32::from_rgb(22, 163, 74));
                    painter.circle_stroke(sp, r + 1.5, Stroke::new(1.2_f32, Color32::from_rgb(34, 197, 94)));
                }
            }
        }
    }

    // 4.5. Interactive Flex Bend & Shorten Handles (3 cyan interior bend points + 1 amber endpoint handle)
    let selected_flex_handles: Option<(String, [Pos2; 4])> = if selected.len() == 1 {
        selected.iter().next().and_then(|sid| {
            let p = layout.pieces.iter().find(|pc| &pc.id == sid)?;
            if crate::catalog::is_flex_id(catalogs, &p.catalog_id) {
                let wh = crate::catalog::flex_bend_handles_world(p)?;
                let mut sh = [Pos2::ZERO; 4];
                for i in 0..4 {
                    sh[i] = camera.world_to_screen(rect, wh[i].x, wh[i].y);
                    let (r, fill) = if i < 3 {
                        (6.5_f32, Color32::from_rgb(56, 189, 248)) // 3 interior bend points
                    } else {
                        (7.5_f32, Color32::from_rgb(245, 158, 11)) // endpoint length/swing handle
                    };
                    painter.circle_filled(sh[i], r, fill);
                    painter.circle_stroke(sh[i], r + 1.2, Stroke::new(1.8_f32, Color32::WHITE));
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

    // 5. Primary click/drag: CAD Board Sketcher OR '+' Quick-Add OR Brush OR Track/Model/Board manipulation
    if sketcher.active {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        let disable_snap = ui.input(|i| i.modifiers.shift || i.modifiers.alt);
        let mut refs: Vec<Vec2> = sketcher.points.clone();
        refs.extend_from_slice(&sketcher.edit_points);
        for b in board_tools::get_baseboards_slice(layout) {
            if b.enabled {
                refs.extend(board_tools::get_baseboard_polygon(&b));
            }
        }
        let pointer_world = hover_pos
            .or_else(|| response.interact_pointer_pos())
            .filter(|sp| rect.contains(*sp))
            .map(|sp| {
                let raw = camera.screen_to_world(rect, sp);
                sketcher.compute_snapped_point(raw, &refs, disable_snap)
            });

        let (enter_pressed, backspace_pressed) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Backspace),
            )
        });
        if backspace_pressed && sketcher.mode == SketchMode::Poly {
            sketcher.points.pop();
        }
        if enter_pressed {
            match sketcher.mode {
                SketchMode::Poly if sketcher.points.len() >= 3 => {
                    let pts = sketcher.points.clone();
                    if let Some(bid) = sketcher.commit_points(layout, next_piece_seq, &pts, false, true) {
                        *active_board_id = Some(bid);
                    }
                    sketcher.stop();
                    interaction.push_undo = true;
                    interaction.changed = true;
                }
                SketchMode::Edit => {
                    sketcher.stop();
                }
                _ => {}
            }
        }

        match sketcher.mode {
            SketchMode::Poly => {
                if response.double_clicked_by(egui::PointerButton::Primary) && sketcher.points.len() >= 3 {
                    let pts = sketcher.points.clone();
                    if let Some(bid) = sketcher.commit_points(layout, next_piece_seq, &pts, false, true) {
                        *active_board_id = Some(bid);
                    }
                    sketcher.stop();
                    interaction.push_undo = true;
                    interaction.changed = true;
                } else if response.clicked_by(egui::PointerButton::Primary) {
                    if let Some(sp) = response.interact_pointer_pos() {
                        let raw = camera.screen_to_world(rect, sp);
                        let snapped = sketcher.compute_snapped_point(raw, &refs, disable_snap);
                        let close_on_first = sketcher.points.len() >= 3
                            && sp.distance(camera.world_to_screen(
                                rect,
                                sketcher.points[0].x,
                                sketcher.points[0].y,
                            )) <= 16.0;
                        if close_on_first {
                            let pts = sketcher.points.clone();
                            if let Some(bid) =
                                sketcher.commit_points(layout, next_piece_seq, &pts, false, true)
                            {
                                *active_board_id = Some(bid);
                            }
                            sketcher.stop();
                            interaction.push_undo = true;
                            interaction.changed = true;
                        } else {
                            let dup = sketcher.points.last().map_or(false, |last| {
                                (last.x - snapped.x).hypot(last.y - snapped.y) < 1.0
                            });
                            if !dup {
                                sketcher.points.push(snapped);
                            }
                        }
                    }
                } else if response.clicked_by(egui::PointerButton::Secondary) {
                    if !sketcher.points.is_empty() {
                        sketcher.points.pop();
                    } else {
                        sketcher.stop();
                    }
                }
            }
            SketchMode::Rect => {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    if let Some(sp) = response.interact_pointer_pos() {
                        let raw = camera.screen_to_world(rect, sp);
                        let snapped = sketcher.compute_snapped_point(raw, &refs, disable_snap);
                        sketcher.rect_start = Some(snapped);
                        sketcher.dragging_rect = true;
                    }
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) && sketcher.dragging_rect {
                    if let (Some(a), Some(sp)) =
                        (sketcher.rect_start.take(), response.interact_pointer_pos())
                    {
                        let raw = camera.screen_to_world(rect, sp);
                        let b = sketcher.compute_snapped_point(raw, &refs, disable_snap);
                        if (b.x - a.x).abs() >= 50.0 && (b.y - a.y).abs() >= 50.0 {
                            let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
                            let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
                            let pts = vec![
                                Vec2 { x: x0, y: y0 },
                                Vec2 { x: x1, y: y0 },
                                Vec2 { x: x1, y: y1 },
                                Vec2 { x: x0, y: y1 },
                            ];
                            if let Some(bid) =
                                sketcher.commit_points(layout, next_piece_seq, &pts, true, true)
                            {
                                *active_board_id = Some(bid);
                            }
                            sketcher.stop();
                            interaction.push_undo = true;
                            interaction.changed = true;
                        }
                    }
                    sketcher.dragging_rect = false;
                }
            }
            SketchMode::Edit => {
                let mut inserted = false;
                if response.drag_started_by(egui::PointerButton::Primary) {
                    if let Some(sp) = response.interact_pointer_pos() {
                        if let Some(vi) = sketcher.edit_points.iter().position(|pt| {
                            sp.distance(camera.world_to_screen(rect, pt.x, pt.y)) <= 14.0
                        }) {
                            sketcher.dragging_vertex = Some(vi);
                            interaction.push_undo = true;
                        } else if sketcher.edit_points.len() >= 3 {
                            let n = sketcher.edit_points.len();
                            for ei in 0..n {
                                let a = sketcher.edit_points[ei];
                                let b = sketcher.edit_points[(ei + 1) % n];
                                let mid = Vec2 {
                                    x: (a.x + b.x) * 0.5,
                                    y: (a.y + b.y) * 0.5,
                                };
                                if sp.distance(camera.world_to_screen(rect, mid.x, mid.y)) <= 12.0 {
                                    sketcher.edit_points.insert(ei + 1, mid);
                                    sketcher.dragging_vertex = Some(ei + 1);
                                    let pts = sketcher.edit_points.clone();
                                    sketcher.commit_points(layout, next_piece_seq, &pts, false, false);
                                    interaction.push_undo = true;
                                    interaction.changed = true;
                                    inserted = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                if !inserted && response.clicked_by(egui::PointerButton::Primary) && sketcher.edit_points.len() >= 3 {
                    if let Some(sp) = response.interact_pointer_pos() {
                        let on_v = sketcher.edit_points.iter().any(|pt| {
                            sp.distance(camera.world_to_screen(rect, pt.x, pt.y)) <= 14.0
                        });
                        if !on_v {
                            let n = sketcher.edit_points.len();
                            for ei in 0..n {
                                let a = sketcher.edit_points[ei];
                                let b = sketcher.edit_points[(ei + 1) % n];
                                let mid = Vec2 {
                                    x: (a.x + b.x) * 0.5,
                                    y: (a.y + b.y) * 0.5,
                                };
                                if sp.distance(camera.world_to_screen(rect, mid.x, mid.y)) <= 12.0 {
                                    sketcher.edit_points.insert(ei + 1, mid);
                                    let pts = sketcher.edit_points.clone();
                                    sketcher.commit_points(layout, next_piece_seq, &pts, false, false);
                                    interaction.push_undo = true;
                                    interaction.changed = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                if let Some(vi) = sketcher.dragging_vertex {
                    if response.dragged_by(egui::PointerButton::Primary) {
                        if let Some(sp) = response.interact_pointer_pos() {
                            let raw = camera.screen_to_world(rect, sp);
                            let mut other_refs = refs;
                            if vi < sketcher.edit_points.len() {
                                let p = sketcher.edit_points[vi];
                                other_refs.retain(|r| (r.x - p.x).hypot(r.y - p.y) > 1.0);
                            }
                            let snapped =
                                sketcher.compute_snapped_point(raw, &other_refs, disable_snap);
                            if let Some(slot) = sketcher.edit_points.get_mut(vi) {
                                *slot = snapped;
                                let pts = sketcher.edit_points.clone();
                                sketcher.commit_points(layout, next_piece_seq, &pts, false, false);
                                interaction.changed = true;
                            }
                        }
                    }
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) {
                    sketcher.dragging_vertex = None;
                }
                if response.clicked_by(egui::PointerButton::Secondary) && sketcher.edit_points.len() > 3 {
                    if let Some(sp) = response.interact_pointer_pos() {
                        if let Some(vi) = sketcher.edit_points.iter().position(|pt| {
                            sp.distance(camera.world_to_screen(rect, pt.x, pt.y)) <= 14.0
                        }) {
                            sketcher.edit_points.remove(vi);
                            let pts = sketcher.edit_points.clone();
                            sketcher.commit_points(layout, next_piece_seq, &pts, false, false);
                            interaction.push_undo = true;
                            interaction.changed = true;
                        }
                    }
                }
            }
        }
        draw_sketcher_overlay(&painter, rect, camera, sketcher, pointer_world);
    } else if terrain_brush.active && response.clicked_by(egui::PointerButton::Primary) {
        if let Some(sp) = response.interact_pointer_pos() {
            let wp = camera.screen_to_world(rect, sp);
            interaction.push_undo = true;
            let active_lid = active_layer_id.to_string();
            let mut chg = terrain_brush.start_stroke(wp, layout);
            if let Some(created_id) =
                terrain_brush.finish_stroke(layout, next_piece_seq, Some(active_lid.as_str()), is_de)
            {
                selected.clear();
                selected.insert(created_id);
                chg = true;
            }
            interaction.changed |= chg;
        }
    } else if !terrain_brush.active && !measure_mode && response.clicked_by(egui::PointerButton::Primary) {
        if let Some(sp) = response.interact_pointer_pos() {
            if let Some((pid, cidx, _, _)) = plus_buttons
                .iter()
                .find(|(_, _, ppos, _)| sp.distance(*ppos) <= 13.5)
            {
                interaction.quick_add_at = Some((pid.clone(), *cidx));
            } else {
                let wp = camera.screen_to_world(rect, sp);
                let hit = find_building_at(layout, wp)
                    .or_else(|| find_piece_at(layout, wp, 22.0 / (camera.zoom as f64).max(0.25)))
                    .or_else(|| find_terrain_at(layout, wp));
                let shift = ui.input(|i| i.modifiers.shift);
                if !shift {
                    selected.clear();
                }
                if let Some(id) = hit {
                    if shift && selected.contains(&id) {
                        selected.remove(&id);
                    } else {
                        selected.insert(id);
                    }
                } else if let Some((bid, _)) = find_board_drag_hit(
                    layout,
                    wp,
                    sp,
                    camera.zoom,
                    &board_header_rects,
                    true,
                ) {
                    *active_board_id = Some(bid);
                }
            }
        }
    }

    if !sketcher.active {
        // 6. Primary drag: Terrain Brush OR Flex handle OR Measure tool OR drag tracks/models OR drag baseboard OR marquee
    if response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(sp) = response.interact_pointer_pos() {
            let wp = camera.screen_to_world(rect, sp);
            let on_plus_btn = !terrain_brush.active
                && !measure_mode
                && plus_buttons
                    .iter()
                    .any(|(_, _, ppos, _)| sp.distance(*ppos) <= 13.5);
            if terrain_brush.active {
                interaction.push_undo = true;
                if terrain_brush.start_stroke(wp, layout) {
                    interaction.changed = true;
                }
            } else if !on_plus_btn {
                if let Some((ref fid, handles)) = selected_flex_handles {
                    for (hi, &hpos) in handles.iter().enumerate() {
                        if sp.distance(hpos) <= 15.0 {
                            *dragging_flex_handle = Some((fid.clone(), hi));
                            interaction.push_undo = true;
                            break;
                        }
                    }
                }
                if dragging_flex_handle.is_none() {
                    if let Some((ref rot_mid, rot_hpos)) = selected_model_rot_handle {
                        if sp.distance(rot_hpos) <= 14.0 {
                            *dragging_model_rot = Some(rot_mid.clone());
                            interaction.push_undo = true;
                        }
                    }
                }
                if dragging_flex_handle.is_none() && dragging_model_rot.is_none() {
                    if measure_mode {
                        *measure_points = Some((wp, wp));
                    } else if let Some(hit) = find_building_at(layout, wp)
                        .or_else(|| find_piece_at(layout, wp, 22.0 / (camera.zoom as f64).max(0.25)))
                        .or_else(|| find_terrain_at(layout, wp))
                    {
                        if !selected.contains(&hit) {
                            selected.clear();
                            selected.insert(hit);
                        }
                        // Clear stale connections pointing to or from dragged pieces so re-snapping works!
                        for p in &mut layout.pieces {
                            let is_sel = selected.contains(&p.id);
                            for c in &mut p.connectors {
                                if is_sel
                                    || c.connected_to
                                        .as_ref()
                                        .map_or(false, |r| selected.contains(&r.piece_id))
                                {
                                    c.connected_to = None;
                                }
                            }
                        }
                        *dragging_active = true;
                        *drag_last_world = Some(wp);
                        interaction.push_undo = true;
                    } else if let Some((bid, bounds)) = find_board_drag_hit(
                        layout,
                        wp,
                        sp,
                        camera.zoom,
                        &board_header_rects,
                        ui.input(|i| i.modifiers.alt),
                    ) {
                        *active_board_id = Some(bid.clone());
                        *dragging_board = Some((bid, wp, bounds.x, bounds.y));
                        interaction.push_undo = true;
                    } else {
                        *marquee_start = Some(sp);
                    }
                }
            }
        }
    }

    if terrain_brush.active && terrain_brush.is_drawing {
        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(sp) = response.interact_pointer_pos() {
                let wp = camera.screen_to_world(rect, sp);
                if terrain_brush.move_stroke(wp, layout) {
                    interaction.changed = true;
                }
            }
        }
    } else if let Some((ref fid, handle_idx)) = dragging_flex_handle.clone() {
        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(sp) = response.interact_pointer_pos() {
                let wp = camera.screen_to_world(rect, sp);
                if let Some(p) = layout.pieces.iter_mut().find(|pc| &pc.id == fid) {
                    if handle_idx < 3 {
                        // Drag one of the 3 interior bend points (25%, 50%, 75%)
                        crate::catalog::apply_flex_spline_offset(p, handle_idx, wp);
                        interaction.changed = true;
                    } else if p.connectors.len() >= 2 {
                        // Drag endpoint handle (shorten/lengthen + bend arc)
                        let x0 = p.connectors[0].x;
                        let y0 = p.connectors[0].y;
                        let entry_heading = norm_angle_deg(p.connectors[0].angle + 180.0);
                        let local = rotate(Vec2 { x: wp.x - x0, y: wp.y - y0 }, -entry_heading);
                        let chord = local.x.hypot(local.y).max(25.0);
                        let alpha = local.y.atan2(local.x.max(5.0));
                        let bend_deg = (2.0 * alpha).to_degrees().clamp(-175.0, 175.0);
                        let arc_len = if alpha.abs() < 0.01 {
                            chord
                        } else {
                            chord * (alpha / alpha.sin())
                        };
                        crate::catalog::apply_flex_geometry(p, catalogs, arc_len, bend_deg);
                        interaction.changed = true;
                    }
                }
            }
        }
    } else if measure_mode && response.dragged_by(egui::PointerButton::Primary) {
        if let (Some(sp), Some((a, _))) = (response.interact_pointer_pos(), *measure_points) {
            *measure_points = Some((a, camera.screen_to_world(rect, sp)));
        }
    } else if marquee_start.is_some() && response.dragged_by(egui::PointerButton::Primary) {
        if let (Some(start), Some(cur)) = (*marquee_start, response.interact_pointer_pos()) {
            let mrect = Rect::from_two_pos(start, cur);
            painter.rect_filled(mrect, 2.0, Color32::from_rgba_unmultiplied(56, 189, 248, 38));
            painter.rect_stroke(mrect, 2.0, Stroke::new(1.2_f32, Color32::from_rgb(56, 189, 248)), egui::StrokeKind::Inside);
        }
    } else if let Some(ref rot_mid) = dragging_model_rot.clone() {
        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(sp) = response.interact_pointer_pos() {
                let wp = camera.screen_to_world(rect, sp);
                let free_rot = ui.input(|i| i.modifiers.shift);
                if let Some(models) = &mut layout.models {
                    if let Some(m) = models.iter_mut().find(|m| &m.id == rot_mid) {
                        let raw_deg =
                            (wp.y - m.transform.y).atan2(wp.x - m.transform.x).to_degrees() + 90.0;
                        let deg = if free_rot {
                            raw_deg
                        } else {
                            (raw_deg / 5.0).round() * 5.0
                        };
                        m.transform.rotation = norm_angle_deg(deg);
                        interaction.changed = true;
                    }
                }
            }
        }
    } else if let Some((ref bid, start_wp, orig_x, orig_y)) = dragging_board.clone() {
        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(sp) = response.interact_pointer_pos() {
                let cur_w = camera.screen_to_world(rect, sp);
                let cand_x = orig_x + (cur_w.x - start_wp.x);
                let cand_y = orig_y + (cur_w.y - start_wp.y);
                let disable_snap = !snapping_enabled || ui.input(|i| i.modifiers.alt);
                let others: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
                    .into_iter()
                    .filter(|b| b.enabled && b.id.as_deref() != Some(bid.as_str()))
                    .collect();
                let boards_mut = board_tools::ensure_baseboards(layout);
                if let Some(b) = boards_mut.iter_mut().find(|b| b.id.as_deref() == Some(bid.as_str())) {
                    b.position_mode = "custom".into();
                    if disable_snap {
                        b.custom_x = Some(cand_x.round());
                        b.custom_y = Some(cand_y.round());
                        board_snap_guides.clear();
                    } else {
                        let threshold = 35.0 / (camera.zoom as f64).clamp(0.25, 2.0);
                        let snap = board_tools::snap_baseboard(b, cand_x, cand_y, &others, threshold);
                        b.custom_x = Some(if snap.snapped_x { snap.x } else { cand_x }.round());
                        b.custom_y = Some(if snap.snapped_y { snap.y } else { cand_y }.round());
                        *board_snap_guides = snap.guides;
                    }
                    board_tools::sync_primary_baseboard(layout);
                    interaction.changed = true;
                }
            }
        }
    } else if *dragging_active && response.dragged_by(egui::PointerButton::Primary) {
        if let (Some(sp), Some(last_w)) = (response.interact_pointer_pos(), *drag_last_world) {
            let cur_w = camera.screen_to_world(rect, sp);
            let dx = cur_w.x - last_w.x;
            let dy = cur_w.y - last_w.y;
            if dx.abs() > 1e-4 || dy.abs() > 1e-4 {
                for p in &mut layout.pieces {
                    if selected.contains(&p.id) {
                        p.transform.x += dx;
                        p.transform.y += dy;
                        for c in &mut p.connectors {
                            c.x += dx;
                            c.y += dy;
                            c.connected_to = None;
                        }
                    }
                }
                if let Some(models) = &mut layout.models {
                    for m in models {
                        if selected.contains(&m.id) {
                            m.transform.x += dx;
                            m.transform.y += dy;
                        }
                    }
                }
                *drag_last_world = Some(cur_w);
                interaction.changed = true;
            }
        }
        // Live magnetic snap preview ring while dragging
        if snapping_enabled {
            let snap_radius = (42.0 / (camera.zoom as f64).max(0.2)).clamp(35.0, 95.0);
            let preview_pair = find_best_snap_candidate(layout, catalogs, selected, snap_radius)
                .map(|cand| (Vec2 { x: cand.src_x, y: cand.src_y }, Vec2 { x: cand.dst_x, y: cand.dst_y }))
                .or_else(|| {
                    crate::building_snap::find_best_track_to_building_snap(layout, catalogs, selected, snap_radius)
                        .map(|bc| (Vec2 { x: bc.src_x, y: bc.src_y }, Vec2 { x: bc.dst_x, y: bc.dst_y }))
                })
                .or_else(|| crate::building_snap::preview_selected_model_snap(layout, catalogs, selected));
            if let Some((src, dst)) = preview_pair {
                let s1 = camera.world_to_screen(rect, src.x, src.y);
                let s2 = camera.world_to_screen(rect, dst.x, dst.y);
                let cyan = Color32::from_rgb(56, 189, 248);
                painter.line_segment([s1, s2], Stroke::new(1.8_f32, cyan));
                painter.circle_stroke(s2, 10.0, Stroke::new(2.2_f32, cyan));
                painter.circle_filled(s2, 4.0, Color32::from_rgb(34, 197, 94));
            }
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        if terrain_brush.is_drawing {
            let was_erase = terrain_brush.mode == TerrainBrushMode::Erase;
            let active_lid = active_layer_id.to_string();
            if let Some(created_id) =
                terrain_brush.finish_stroke(layout, next_piece_seq, Some(active_lid.as_str()), is_de)
            {
                selected.clear();
                selected.insert(created_id);
                interaction.changed = true;
            } else if was_erase {
                interaction.changed = true;
            }
        }
        if dragging_flex_handle.take().is_some() {
            railbench_core::snapping::update_layout_connections(layout, 4.0, 35.0, 1.0);
            crate::catalog::disconnect_cross_type_connections(layout, catalogs);
            interaction.changed = true;
        }
        if let (Some(start), Some(end)) = (marquee_start.take(), response.interact_pointer_pos()) {
            let mrect = Rect::from_two_pos(start, end);
            let shift = ui.input(|i| i.modifiers.shift);
            if !shift {
                selected.clear();
            }
            for p in &layout.pieces {
                if !is_layer_editable(layout, p.layer_id.as_deref()) {
                    continue;
                }
                let any_in = p
                    .connectors
                    .iter()
                    .any(|c| mrect.contains(camera.world_to_screen(rect, c.x, c.y)));
                if any_in {
                    selected.insert(p.id.clone());
                }
            }
            if let Some(models) = &layout.models {
                for m in models {
                    if !is_layer_editable(layout, m.layer_id.as_deref()) {
                        continue;
                    }
                    let sp = camera.world_to_screen(rect, m.transform.x, m.transform.y);
                    if mrect.contains(sp) {
                        selected.insert(m.id.clone());
                    }
                }
            }
        }
        if dragging_model_rot.take().is_some() {
            if snapping_enabled && !selected.is_empty() {
                crate::building_snap::snap_selected_models(layout, catalogs, selected);
            }
            interaction.changed = true;
        }
        if dragging_board.take().is_some() {
            board_snap_guides.clear();
            interaction.changed = true;
        }
        if *dragging_active {
            *dragging_active = false;
            *drag_last_world = None;

            // Magnetic rigid-body snap for single OR multi-piece selection + building/turntable snap
            if snapping_enabled && !selected.is_empty() {
                let snap_radius = (42.0 / (camera.zoom as f64).max(0.2)).clamp(35.0, 95.0);
                let mut snapped = snap_selected_pieces(layout, catalogs, selected, snap_radius);
                if !snapped {
                    snapped = crate::building_snap::snap_selected_pieces_to_buildings(
                        layout,
                        catalogs,
                        selected,
                        snap_radius,
                    );
                }
                snapped |= crate::building_snap::snap_selected_models(layout, catalogs, selected);
                if snapped {
                    interaction.changed = true;
                }
            }
            railbench_core::snapping::update_layout_connections(layout, 4.0, 35.0, 1.0);
            crate::catalog::disconnect_cross_type_connections(layout, catalogs);
            interaction.changed = true;
        }
    }
    } // !sketcher.active

    // 7. Measure tool overlay
    if measure_mode {
        if let Some((a, b)) = *measure_points {
            let sa = camera.world_to_screen(rect, a.x, a.y);
            let sb = camera.world_to_screen(rect, b.x, b.y);
            let dist_mm = (b.x - a.x).hypot(b.y - a.y);
            let col = Color32::from_rgb(245, 158, 11);
            painter.line_segment([sa, sb], Stroke::new(2.0_f32, col));
            painter.circle_filled(sa, 4.0, col);
            painter.circle_filled(sb, 4.0, col);
            let mid = Pos2::new((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5 - 14.0);
            let lbl = format!("{dist_mm:.1} mm ({:.1} cm)", dist_mm / 10.0);
            let bg_r = Rect::from_center_size(mid, EguiVec2::new(140.0, 22.0));
            painter.rect_filled(bg_r, 6.0, Color32::from_rgba_unmultiplied(15, 23, 42, 230));
            painter.rect_stroke(bg_r, 6.0, Stroke::new(1.0_f32, col), egui::StrokeKind::Inside);
            painter.text(mid, egui::Align2::CENTER_CENTER, lbl, FontId::proportional(11.5), Color32::WHITE);
        }
    }

    // 7.5. Catalog Drag-and-Drop live track preview + drop handling
    if let Some(entry) = dragging_catalog {
        if let Some(pointer) = ui.input(|i| i.pointer.latest_pos()) {
            if rect.contains(pointer) {
                let wp = camera.screen_to_world(rect, pointer);
                let locals = crate::catalog::local_connectors(entry);
                let (cx, cy) = if locals.is_empty() {
                    (0.0, 0.0)
                } else {
                    let sx: f64 = locals.iter().map(|c| c.x).sum();
                    let sy: f64 = locals.iter().map(|c| c.y).sum();
                    (sx / locals.len() as f64, sy / locals.len() as f64)
                };
                let ghost = crate::catalog::create_piece(
                    "__ghost__".into(),
                    entry,
                    railbench_core::types::Transform {
                        x: wp.x - cx,
                        y: wp.y - cy,
                        rotation: 0.0,
                    },
                    None,
                );
                let snap_radius = (42.0 / (camera.zoom as f64).max(0.2)).clamp(35.0, 95.0);

                // Draw live ghost track at cursor
                for branch in piece_branches_world(&ghost) {
                    if branch.len() >= 2 {
                        let spts: Vec<Pos2> = branch
                            .iter()
                            .map(|p| camera.world_to_screen(rect, p.x, p.y))
                            .collect();
                        painter.add(egui::Shape::line(
                            spts,
                            Stroke::new(
                                ((entry.gauge.unwrap_or(16.5) / 16.5) * 34.0 * camera.zoom as f64).max(4.0) as f32,
                                Color32::from_rgba_unmultiplied(56, 189, 248, 120),
                            ),
                        ));
                    }
                }

                // Live snap target ring if near an open connector of the SAME type (track-to-track or road-to-road)
                if snapping_enabled {
                    let ghost_is_road = entry.is_road == Some(true) || crate::catalog::is_road_id(catalogs, &entry.id);
                    let mut best_snap: Option<(f64, f64, f64, f64, f64)> = None;
                    for c1 in &ghost.connectors {
                        for other in &layout.pieces {
                            if !is_layer_visible(layout, other.layer_id.as_deref()) {
                                continue;
                            }
                            if crate::catalog::is_road_id(catalogs, &other.catalog_id) != ghost_is_road {
                                continue;
                            }
                            for c2 in &other.connectors {
                                if c2.connected_to.is_some() {
                                    continue;
                                }
                                let d = (c1.x - c2.x).hypot(c1.y - c2.y);
                                if d <= snap_radius && best_snap.as_ref().map_or(true, |b| d < b.4) {
                                    best_snap = Some((c1.x, c1.y, c2.x, c2.y, d));
                                }
                            }
                        }
                    }
                    if let Some((sx, sy, dx, dy, _)) = best_snap {
                        let p1 = camera.world_to_screen(rect, sx, sy);
                        let p2 = camera.world_to_screen(rect, dx, dy);
                        let cyan = Color32::from_rgb(56, 189, 248);
                        painter.line_segment([p1, p2], Stroke::new(1.8_f32, cyan));
                        painter.circle_stroke(p2, 10.0, Stroke::new(2.2_f32, cyan));
                        painter.circle_filled(p2, 4.0, Color32::from_rgb(34, 197, 94));
                    }
                }

                if ui.input(|i| i.pointer.primary_released()) {
                    interaction.drop_catalog_at = Some((wp, snap_radius));
                }
            }
        }
    }

    // 8. Top & Left Millimeter Rulers (matches src/ui/ruler.ts)
    if show_rulers {
        draw_rulers(&painter, rect, camera, dark_mode);
    }

    // 9. Right-click context menu (matches src/ui/track-context-menu.ts)
    if !selected.is_empty() {
        let any_track_sel = layout.pieces.iter().any(|p| selected.contains(&p.id));
        let any_turnout_sel = layout.pieces.iter().any(|p| {
            selected.contains(&p.id)
                && find_entry(catalogs, &p.catalog_id).map_or(false, |e| crate::is_turnout_kind(&e.kind))
        });
        let any_signal_sel = layout
            .pieces
            .iter()
            .any(|p| selected.contains(&p.id) && p.has_signal == Some(true));
        let ctx_sig_side = layout
            .pieces
            .iter()
            .find(|p| selected.contains(&p.id) && p.has_signal == Some(true))
            .and_then(|p| p.signal_side.as_deref())
            .unwrap_or("right")
            .to_string();
        let ctx_sig_dir = layout
            .pieces
            .iter()
            .find(|p| selected.contains(&p.id) && p.has_signal == Some(true))
            .and_then(|p| p.signal_direction.as_deref())
            .unwrap_or("forward")
            .to_string();
        let ctx_layers: Vec<(String, String)> = layout
            .layers
            .as_ref()
            .map(|ls| ls.iter().map(|l| (l.id.clone(), l.name.clone())).collect())
            .unwrap_or_else(|| vec![("layer-1".into(), "Ebene 0".into())]);

        response.context_menu(|ui| {
            if ui.button(format!("↺ Rotate -{rotation_step}°")).clicked() {
                interaction.rotate_delta = Some(-rotation_step);
                ui.close_menu();
            }
            if ui.button(format!("↻ Rotate +{rotation_step}°")).clicked() {
                interaction.rotate_delta = Some(rotation_step);
                ui.close_menu();
            }
            if ui.button("Copy (Ctrl+C)").clicked() {
                interaction.copy_selected = true;
                ui.close_menu();
            }
            if ui.button("Duplicate (Ctrl+D)").clicked() {
                interaction.duplicate_selected = true;
                ui.close_menu();
            }
            ui.menu_button("Move to Layer", |ui| {
                for (lid, lname) in &ctx_layers {
                    if ui.button(lname).clicked() {
                        interaction.move_to_layer = Some(lid.clone());
                        ui.close_menu();
                    }
                }
            });
            if any_track_sel {
                ui.separator();
                if ui.button("Select Connected Tracks").clicked() {
                    interaction.select_connected = true;
                    ui.close_menu();
                }
                if ui.button("Select Whole Layer").clicked() {
                    interaction.select_layer = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Toggle Power Feeder (F)").clicked() {
                    interaction.toggle_feeder = true;
                    ui.close_menu();
                }
                if ui.button("Toggle Isolator (I)").clicked() {
                    interaction.toggle_isolator = true;
                    ui.close_menu();
                }
                if ui.button("Toggle Catenary (O)").clicked() {
                    interaction.toggle_catenary = true;
                    ui.close_menu();
                }
                if ui.button("Toggle Catenary Feeder (Shift+F)").clicked() {
                    interaction.toggle_catenary_feeder = true;
                    ui.close_menu();
                }
                if ui.button("Toggle Catenary Isolator (Shift+I)").clicked() {
                    interaction.toggle_catenary_isolator = true;
                    ui.close_menu();
                }
                if ui.button("Toggle Signal (Shift+S)").clicked() {
                    interaction.toggle_signal = true;
                    ui.close_menu();
                }
                if any_signal_sel {
                    if ui.button(format!("Signal Side: {ctx_sig_side}")).clicked() {
                        interaction.cycle_signal_side = true;
                        ui.close_menu();
                    }
                    if ui.button(format!("Signal Direction: {ctx_sig_dir}")).clicked() {
                        interaction.cycle_signal_direction = true;
                        ui.close_menu();
                    }
                }
                if any_turnout_sel {
                    if ui.button("Toggle Turnout Lantern (Shift+L)").clicked() {
                        interaction.toggle_lantern = true;
                        ui.close_menu();
                    }
                    if ui.button("Toggle Turnout Decoder (Shift+D)").clicked() {
                        interaction.toggle_decoder = true;
                        ui.close_menu();
                    }
                }
                if ui.button("Toggle Bridge (B)").clicked() {
                    interaction.toggle_bridge = true;
                    ui.close_menu();
                }
                if ui.button("Toggle Tunnel (T)").clicked() {
                    interaction.toggle_tunnel = true;
                    ui.close_menu();
                }
            }
            ui.separator();
            if ui.button("Delete Selected (Del)").clicked() {
                interaction.delete_selected = true;
                ui.close_menu();
            }
        });
    }

    interaction
}

fn draw_grid(painter: &egui::Painter, rect: Rect, camera: &Camera2D, dark_mode: bool) {
    let top_left = camera.screen_to_world(rect, rect.min);
    let bot_right = camera.screen_to_world(rect, rect.max);

    let fine_color = if dark_mode {
        Color32::from_rgba_unmultiplied(148, 163, 184, 18)
    } else {
        Color32::from_rgba_unmultiplied(71, 85, 105, 22)
    };
    let major_color = if dark_mode {
        Color32::from_rgba_unmultiplied(148, 163, 184, 42)
    } else {
        Color32::from_rgba_unmultiplied(71, 85, 105, 50)
    };

    let draw_step = |step: f64, color: Color32, width: f32| {
        let sx = (top_left.x / step).floor() as i64;
        let ex = (bot_right.x / step).ceil() as i64;
        let sy = (top_left.y / step).floor() as i64;
        let ey = (bot_right.y / step).ceil() as i64;
        if (ex - sx) > 400 || (ey - sy) > 400 {
            return;
        }
        for ix in sx..=ex {
            let x = camera.world_to_screen(rect, ix as f64 * step, 0.0).x;
            painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], Stroke::new(width, color));
        }
        for iy in sy..=ey {
            let y = camera.world_to_screen(rect, 0.0, iy as f64 * step).y;
            painter.line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)], Stroke::new(width, color));
        }
    };

    if camera.zoom >= 0.25 {
        draw_step(50.0, fine_color, 1.0);
    }
    draw_step(250.0, major_color, 1.0);

    // Origin crosshair
    let o = camera.world_to_screen(rect, 0.0, 0.0);
    let axis_col = Color32::from_rgba_unmultiplied(56, 189, 248, 90);
    painter.line_segment([Pos2::new(o.x - 14.0, o.y), Pos2::new(o.x + 14.0, o.y)], Stroke::new(1.5_f32, axis_col));
    painter.line_segment([Pos2::new(o.x, o.y - 14.0), Pos2::new(o.x, o.y + 14.0)], Stroke::new(1.5_f32, axis_col));
}

fn board_style_colors(style: &str) -> (Color32, Color32, Color32) {
    match style {
        "dark-wood" => (
            Color32::from_rgb(92, 64, 51),
            Color32::from_rgb(56, 36, 28),
            Color32::from_rgb(72, 49, 39),
        ),
        "plywood" => (
            Color32::from_rgb(212, 180, 131),
            Color32::from_rgb(156, 122, 84),
            Color32::from_rgb(186, 152, 106),
        ),
        "slate" => (
            Color32::from_rgb(74, 85, 104),
            Color32::from_rgb(45, 55, 72),
            Color32::from_rgb(58, 68, 86),
        ),
        "white" => (
            Color32::from_rgb(241, 245, 249),
            Color32::from_rgb(203, 213, 225),
            Color32::from_rgb(226, 232, 240),
        ),
        "grass" => (
            Color32::from_rgb(61, 107, 46),
            Color32::from_rgb(42, 74, 30),
            Color32::from_rgb(50, 88, 37),
        ),
        _ => (
            Color32::from_rgb(168, 134, 90),
            Color32::from_rgb(92, 61, 46),
            Color32::from_rgb(142, 108, 68),
        ),
    }
}

fn fill_concave_polygon_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    outer: &[Vec2],
    holes: &[Vec<Vec2>],
    fill: Color32,
    outer_stroke: Option<Stroke>,
    hole_stroke: Option<Stroke>,
) {
    let tris = board_tools::triangulate_polygon_with_holes(outer, holes);
    if !tris.is_empty() {
        let mut mesh = egui::Mesh::default();
        for tri in tris {
            let base = mesh.vertices.len() as u32;
            for pt in tri {
                mesh.colored_vertex(camera.world_to_screen(rect, pt.x, pt.y), fill);
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
    if let Some(st) = outer_stroke {
        if outer.len() >= 2 {
            let pts: Vec<Pos2> = outer
                .iter()
                .map(|p| camera.world_to_screen(rect, p.x, p.y))
                .collect();
            painter.add(egui::Shape::closed_line(pts, st));
        }
    }
    if let Some(hst) = hole_stroke {
        for hole in holes {
            if hole.len() >= 2 {
                let pts: Vec<Pos2> = hole
                    .iter()
                    .map(|p| camera.world_to_screen(rect, p.x, p.y))
                    .collect();
                painter.add(egui::Shape::closed_line(pts, hst));
            }
        }
    }
}

fn poly_centroid(poly: &[Vec2]) -> Vec2 {
    if poly.is_empty() {
        return Vec2 { x: 0.0, y: 0.0 };
    }
    let sx: f64 = poly.iter().map(|p| p.x).sum();
    let sy: f64 = poly.iter().map(|p| p.y).sum();
    Vec2 {
        x: sx / poly.len() as f64,
        y: sy / poly.len() as f64,
    }
}

fn find_board_drag_hit(
    layout: &Layout,
    wp: Vec2,
    sp: Pos2,
    zoom: f32,
    header_rects: &[(String, Rect)],
    allow_interior: bool,
) -> Option<(String, BoardBounds)> {
    let boards = board_tools::get_baseboards_slice(layout);
    for (bid, hrect) in header_rects.iter().rev() {
        if hrect.expand(4.0).contains(sp) {
            if let Some(b) = boards.iter().find(|b| b.id.as_deref() == Some(bid.as_str())) {
                return Some((bid.clone(), board_tools::get_baseboard_bounds(b)));
            }
        }
    }
    let tol = 24.0 / (zoom as f64).max(0.25);
    for b in boards.iter().rev() {
        if !b.enabled {
            continue;
        }
        let hit = board_tools::is_point_near_baseboard_edge(wp, b, tol)
            || (allow_interior && board_tools::is_point_inside_baseboard(wp, b));
        if hit {
            let bid = b.id.clone().unwrap_or_else(|| "board-1".into());
            return Some((bid, board_tools::get_baseboard_bounds(b)));
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn draw_baseboards(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    layout: &Layout,
    catalogs: &[Catalog],
    active_board_id: Option<&str>,
    snap_guides: &[BaseboardSnapGuide],
    dark_mode: bool,
) -> Vec<(String, Rect)> {
    let mut header_rects = Vec::new();
    let boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
        .into_iter()
        .filter(|b| b.enabled)
        .collect();
    let multi = boards.len() > 1;
    let first_id = boards.first().and_then(|b| b.id.as_deref());

    for b in &boards {
        let bid = b.id.as_deref().unwrap_or("board-1");
        let is_active = active_board_id.map_or_else(|| Some(bid) == first_id, |aid| aid == bid);
        let poly = board_tools::get_baseboard_polygon(b);
        if poly.len() < 3 {
            continue;
        }
        let bounds = board_tools::get_baseboard_bounds(b);
        let (fill, border_col, grain_col) = board_style_colors(&b.style);
        let cutouts: Vec<TrackCutoutPolygon> =
            board_tools::compute_table_track_cutouts(layout, catalogs, b, 0.0, 20.0);
        let holes: Vec<Vec<Vec2>> = cutouts.into_iter().map(|c| c.points).collect();

        if multi && is_active {
            let pts: Vec<Pos2> = poly
                .iter()
                .map(|p| camera.world_to_screen(rect, p.x, p.y))
                .collect();
            painter.add(egui::Shape::closed_line(
                pts,
                Stroke::new(6.0_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 85)),
            ));
        }

        let border_stroke = if multi && is_active {
            Stroke::new(2.2_f32, Color32::from_rgb(56, 189, 248))
        } else {
            Stroke::new(2.0_f32, border_col)
        };
        fill_concave_polygon_2d(
            painter,
            rect,
            camera,
            &poly,
            &holes,
            fill,
            Some(border_stroke),
            None,
        );

        // Subtle inner inset edge for wood craft look
        if multi || b.style != "white" {
            let _ = grain_col;
        }

        // Draw cutout recess wells + amber outline
        for h in &holes {
            fill_concave_polygon_2d(
                painter,
                rect,
                camera,
                h,
                &[],
                Color32::from_rgba_unmultiplied(15, 23, 42, 175),
                Some(Stroke::new(1.4_f32, Color32::from_rgba_unmultiplied(245, 158, 11, 200))),
                None,
            );
        }

        // Header badge (for clicking/dragging/identifying boards)
        if multi || b.name.is_some() {
            let anchor = poly
                .iter()
                .min_by(|a, b| {
                    (a.y + a.x * 0.25)
                        .partial_cmp(&(b.y + b.x * 0.25))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied()
                .unwrap_or(Vec2 { x: bounds.x, y: bounds.y });
            let sp = camera.world_to_screen(rect, anchor.x, anchor.y);
            let title = b.name.as_deref().unwrap_or("Tisch");
            let pill_w = (title.len() as f32 * 6.5 + 22.0).clamp(58.0, 170.0);
            let hrect = Rect::from_min_size(Pos2::new(sp.x + 6.0, sp.y + 6.0), EguiVec2::new(pill_w, 18.0));
            let pill_bg = if is_active {
                Color32::from_rgba_unmultiplied(14, 116, 144, 225)
            } else {
                Color32::from_rgba_unmultiplied(30, 41, 59, 195)
            };
            painter.rect_filled(hrect, 4.0, pill_bg);
            painter.rect_stroke(
                hrect,
                4.0,
                Stroke::new(1.0_f32, if is_active { Color32::from_rgb(56, 189, 248) } else { border_col }),
                egui::StrokeKind::Inside,
            );
            painter.text(
                hrect.center(),
                egui::Align2::CENTER_CENTER,
                title,
                FontId::proportional(10.0),
                Color32::WHITE,
            );
            header_rects.push((bid.to_string(), hrect));
        }

        // Dimension lines
        if b.show_dimensions != Some(false) && camera.zoom >= 0.08 {
            let dim_col = if dark_mode {
                Color32::from_rgba_unmultiplied(148, 163, 184, 180)
            } else {
                Color32::from_rgba_unmultiplied(51, 65, 85, 190)
            };
            let w0 = camera.world_to_screen(rect, bounds.x, bounds.y);
            let w1 = camera.world_to_screen(rect, bounds.x + bounds.width, bounds.y);
            let d1 = camera.world_to_screen(rect, bounds.x, bounds.y + bounds.depth);
            let off_y = -14.0;
            painter.line_segment(
                [Pos2::new(w0.x, w0.y + off_y), Pos2::new(w1.x, w1.y + off_y)],
                Stroke::new(1.0_f32, dim_col),
            );
            painter.text(
                Pos2::new((w0.x + w1.x) * 0.5, w0.y + off_y - 8.0),
                egui::Align2::CENTER_CENTER,
                format!("{:.0} mm", bounds.width),
                FontId::proportional(10.5),
                dim_col,
            );
            let off_x = -14.0;
            painter.line_segment(
                [Pos2::new(w0.x + off_x, w0.y), Pos2::new(d1.x + off_x, d1.y)],
                Stroke::new(1.0_f32, dim_col),
            );
            painter.text(
                Pos2::new(w0.x + off_x - 24.0, (w0.y + d1.y) * 0.5),
                egui::Align2::CENTER_CENTER,
                format!("{:.0} mm", bounds.depth),
                FontId::proportional(10.5),
                dim_col,
            );
        }
    }

    // Active baseboard magnetic snap lasers
    let laser_col = Color32::from_rgb(34, 211, 238);
    for g in snap_guides {
        let (a, b) = if g.is_x {
            (
                camera.world_to_screen(rect, g.pos, g.from),
                camera.world_to_screen(rect, g.pos, g.to),
            )
        } else {
            (
                camera.world_to_screen(rect, g.from, g.pos),
                camera.world_to_screen(rect, g.to, g.pos),
            )
        };
        painter.line_segment([a, b], Stroke::new(1.8_f32, laser_col));
    }

    header_rects
}

#[allow(clippy::too_many_arguments)]
fn draw_layer_wood_plates(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    layout: &Layout,
    catalogs: &[Catalog],
    layer_id: &str,
    is_active_layer: bool,
    layer_color_hex: Option<&str>,
) {
    let Some(layer) = layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == layer_id))
    else {
        return;
    };
    let lh = layer.height_mm.unwrap_or(0.0);
    let boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
        .into_iter()
        .filter(|b| b.enabled)
        .collect();
    if boards.is_empty() {
        return;
    }

    let accent = parse_hex_color(layer_color_hex)
        .unwrap_or(Color32::from_rgb(56, 189, 248));
    let alpha_fill = if is_active_layer { 175 } else { 75 };
    let alpha_stroke = if is_active_layer { 220 } else { 110 };

    if let Some(plates) = &layer.plates {
        if !plates.is_empty() {
            for plate in plates {
                let target_board = plate
                    .baseboard_id
                    .as_deref()
                    .and_then(|bid| boards.iter().find(|b| b.id.as_deref() == Some(bid)))
                    .or_else(|| boards.first());
                let poly = board_tools::get_layer_plate_polygon(plate, target_board);
                if poly.len() < 3 {
                    continue;
                }
                let pbb = board_tools::layer_plate_as_baseboard(plate, target_board);
                let thick = plate
                    .thickness_mm
                    .or(layer.wood_plate_thickness_mm)
                    .unwrap_or(12.0);
                let holes: Vec<Vec<Vec2>> =
                    board_tools::compute_table_track_cutouts(layout, catalogs, &pbb, lh, thick)
                        .into_iter()
                        .map(|c| c.points)
                        .collect();
                let (base_fill, _, _) =
                    board_style_colors(plate.style.as_deref().unwrap_or("plywood"));
                let fill = Color32::from_rgba_unmultiplied(
                    base_fill.r(),
                    base_fill.g(),
                    base_fill.b(),
                    alpha_fill,
                );
                let stroke = Stroke::new(
                    1.6_f32,
                    Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), alpha_stroke),
                );
                fill_concave_polygon_2d(
                    painter,
                    rect,
                    camera,
                    &poly,
                    &holes,
                    fill,
                    Some(stroke),
                    Some(Stroke::new(1.2_f32, Color32::from_rgb(245, 158, 11))),
                );
                if let Some(name) = &plate.name {
                    if camera.zoom >= 0.14 {
                        let c = poly_centroid(&poly);
                        let sc = camera.world_to_screen(rect, c.x, c.y);
                        painter.text(
                            sc,
                            egui::Align2::CENTER_CENTER,
                            name,
                            FontId::proportional(10.5),
                            Color32::from_rgba_unmultiplied(248, 250, 252, alpha_stroke),
                        );
                    }
                }
            }
            return;
        }
    }

    let mode = layer.wood_plate_mode.as_deref().unwrap_or("none");
    if mode == "table" && lh.abs() >= 1e-6 {
        let thick = layer.wood_plate_thickness_mm.unwrap_or(12.0);
        let (base_fill, _, _) =
            board_style_colors(layer.wood_plate_style.as_deref().unwrap_or("plywood"));
        let fill = Color32::from_rgba_unmultiplied(
            base_fill.r(),
            base_fill.g(),
            base_fill.b(),
            if is_active_layer { 145 } else { 60 },
        );
        let stroke = Stroke::new(
            1.4_f32,
            Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), alpha_stroke),
        );
        for b in &boards {
            let poly = board_tools::get_baseboard_polygon(b);
            let holes: Vec<Vec<Vec2>> =
                board_tools::compute_table_track_cutouts(layout, catalogs, b, lh, thick)
                    .into_iter()
                    .map(|c| c.points)
                    .collect();
            fill_concave_polygon_2d(
                painter,
                rect,
                camera,
                &poly,
                &holes,
                fill,
                Some(stroke),
                Some(Stroke::new(1.2_f32, Color32::from_rgb(245, 158, 11))),
            );
        }
    } else if mode == "tight" {
        let (base_fill, _, _) =
            board_style_colors(layer.wood_plate_style.as_deref().unwrap_or("plywood"));
        let roadbed_col = Color32::from_rgba_unmultiplied(
            base_fill.r(),
            base_fill.g(),
            base_fill.b(),
            if is_active_layer { 150 } else { 65 },
        );
        for p in &layout.pieces {
            if p.layer_id.as_deref().unwrap_or("layer-1") != layer_id || p.connectors.len() < 2 {
                continue;
            }
            let w_scr = (72.0 * camera.zoom).max(6.0);
            let pts: Vec<Pos2> = p
                .connectors
                .iter()
                .take(2)
                .map(|c| camera.world_to_screen(rect, c.x, c.y))
                .collect();
            painter.line_segment([pts[0], pts[1]], Stroke::new(w_scr, roadbed_col));
        }
    }
}

fn draw_sketcher_overlay(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    sketcher: &BoardSketcherState,
    pointer_world: Option<Vec2>,
) {
    let cyan = Color32::from_rgb(56, 189, 248);
    let emerald = Color32::from_rgb(34, 197, 94);
    let guide_col = Color32::from_rgba_unmultiplied(34, 211, 238, 190);

    for (a, b) in &sketcher.active_guides {
        let sa = camera.world_to_screen(rect, a.x, a.y);
        let sb = camera.world_to_screen(rect, b.x, b.y);
        painter.line_segment([sa, sb], Stroke::new(1.3_f32, guide_col));
    }

    match sketcher.mode {
        SketchMode::Poly => {
            if sketcher.points.len() >= 3 {
                fill_concave_polygon_2d(
                    painter,
                    rect,
                    camera,
                    &sketcher.points,
                    &[],
                    Color32::from_rgba_unmultiplied(56, 189, 248, 45),
                    None,
                    None,
                );
            }
            for w in sketcher.points.windows(2) {
                let a = camera.world_to_screen(rect, w[0].x, w[0].y);
                let b = camera.world_to_screen(rect, w[1].x, w[1].y);
                painter.line_segment([a, b], Stroke::new(2.4_f32, cyan));
                let len = (w[1].x - w[0].x).hypot(w[1].y - w[0].y);
                painter.text(
                    Pos2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5 - 10.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{len:.0} mm"),
                    FontId::proportional(10.5),
                    Color32::WHITE,
                );
            }
            if let (Some(last), Some(cur)) = (sketcher.points.last(), pointer_world) {
                let sa = camera.world_to_screen(rect, last.x, last.y);
                let sb = camera.world_to_screen(rect, cur.x, cur.y);
                painter.line_segment([sa, sb], Stroke::new(1.8_f32, Color32::from_rgb(125, 211, 252)));
                let len = (cur.x - last.x).hypot(cur.y - last.y);
                painter.text(
                    Pos2::new((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5 - 10.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{len:.0} mm"),
                    FontId::proportional(10.5),
                    cyan,
                );
            }
            for (idx, pt) in sketcher.points.iter().enumerate() {
                let sp = camera.world_to_screen(rect, pt.x, pt.y);
                let r = if idx == 0 && sketcher.points.len() >= 3 { 7.5 } else { 5.0 };
                let col = if idx == 0 && sketcher.points.len() >= 3 { emerald } else { cyan };
                painter.circle_filled(sp, r, col);
                painter.circle_stroke(sp, r + 1.2, Stroke::new(1.5_f32, Color32::WHITE));
            }
        }
        SketchMode::Rect => {
            if let (Some(a), Some(b)) = (sketcher.rect_start, pointer_world) {
                if sketcher.dragging_rect {
                    let sa = camera.world_to_screen(rect, a.x, a.y);
                    let sb = camera.world_to_screen(rect, b.x, b.y);
                    let r = Rect::from_two_pos(sa, sb);
                    painter.rect_filled(r, 2.0, Color32::from_rgba_unmultiplied(56, 189, 248, 45));
                    painter.rect_stroke(r, 2.0, Stroke::new(2.2_f32, cyan), egui::StrokeKind::Inside);
                    painter.text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{:.0} x {:.0} mm", (b.x - a.x).abs(), (b.y - a.y).abs()),
                        FontId::proportional(12.0),
                        Color32::WHITE,
                    );
                }
            }
        }
        SketchMode::Edit => {
            let pts = &sketcher.edit_points;
            if pts.len() >= 3 {
                let spts: Vec<Pos2> = pts
                    .iter()
                    .map(|p| camera.world_to_screen(rect, p.x, p.y))
                    .collect();
                painter.add(egui::Shape::closed_line(spts, Stroke::new(2.2_f32, cyan)));
                let n = pts.len();
                for ei in 0..n {
                    let a = pts[ei];
                    let b = pts[(ei + 1) % n];
                    let sm = camera.world_to_screen(rect, (a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
                    painter.circle_filled(sm, 6.0, Color32::from_rgb(16, 185, 129));
                    painter.circle_stroke(sm, 6.5, Stroke::new(1.2_f32, Color32::WHITE));
                    painter.text(sm, egui::Align2::CENTER_CENTER, "+", FontId::proportional(11.0), Color32::WHITE);
                }
                for (vi, p) in pts.iter().enumerate() {
                    let sp = camera.world_to_screen(rect, p.x, p.y);
                    let col = if sketcher.dragging_vertex == Some(vi) {
                        Color32::from_rgb(245, 158, 11)
                    } else {
                        cyan
                    };
                    painter.circle_filled(sp, 7.0, col);
                    painter.circle_stroke(sp, 8.0, Stroke::new(1.6_f32, Color32::WHITE));
                }
            }
        }
    }

    if let Some(cur) = pointer_world {
        let sc = camera.world_to_screen(rect, cur.x, cur.y);
        painter.circle_stroke(sc, 6.0, Stroke::new(1.5_f32, cyan));
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_track_piece(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    piece: &TrackPiece,
    all_pieces: &[TrackPiece],
    entry: Option<&CatalogEntry>,
    is_selected: bool,
    is_active_layer: bool,
    color_mode: ColorMode,
    layer_color_hex: Option<&str>,
    show_labels: bool,
    power_sim_enabled: bool,
    piece_power: Option<&PiecePowerResult>,
) {
    let is_road = entry.and_then(|e| e.is_road).unwrap_or(false);
    let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5);
    let road_w_mm = entry.and_then(|e| e.road_width).unwrap_or(68.0);
    let ballast_w = if is_road {
        (road_w_mm * camera.zoom as f64).max(5.0) as f32
    } else {
        ((gauge / 16.5) * 34.0 * camera.zoom as f64).max(3.5) as f32
    };
    let rail_w = (1.6 * camera.zoom).max(1.0);

    let kind = entry.map(|e| e.kind.as_str()).unwrap_or("straight");
    let mut ballast_col = if is_road {
        Color32::from_rgb(55, 65, 81)
    } else if power_sim_enabled {
        if let Some(pr) = piece_power {
            match pr.status {
                PowerStatus::Optimal => Color32::from_rgb(34, 197, 94),
                PowerStatus::Warning => Color32::from_rgb(245, 158, 11),
                PowerStatus::Critical => Color32::from_rgb(239, 68, 68),
                PowerStatus::Disconnected => Color32::from_rgb(71, 85, 105),
            }
        } else if piece.is_isolator == Some(true) {
            Color32::from_rgb(220, 38, 38)
        } else if piece.power_feed == Some(true) {
            Color32::from_rgb(22, 163, 74)
        } else {
            Color32::from_rgb(71, 85, 105)
        }
    } else {
        match color_mode {
            ColorMode::Off => Color32::from_rgb(71, 85, 105),
            ColorMode::Layers => parse_hex_color(layer_color_hex).unwrap_or(Color32::from_rgb(71, 85, 105)),
            ColorMode::Types => match kind {
                "curve" => Color32::from_rgb(37, 99, 235),
                "turnout-left" | "turnout-right" | "three-way" | "double-slip" | "curved-turnout-left" | "curved-turnout-right" => {
                    Color32::from_rgb(217, 119, 6)
                }
                "flex" => Color32::from_rgb(8, 145, 178),
                "crossing" => Color32::from_rgb(147, 51, 234),
                _ => Color32::from_rgb(100, 116, 139),
            },
        }
    };
    if is_selected {
        // Selected tracks are noticeably brighter ("heller") than unselected ones
        ballast_col = brighten_color(ballast_col, 0.38);
    } else if !is_active_layer {
        // Inactive layers are desaturated & slightly dimmed
        ballast_col = desaturate_color(ballast_col, 0.80, 0.72);
    }
    if piece.has_tunnel == Some(true) {
        ballast_col = Color32::from_rgba_unmultiplied(ballast_col.r(), ballast_col.g(), ballast_col.b(), 120);
    }

    // Build all centerline branches (handles curves, curved turnouts, 3-way, crossings, flextracks, turntables)
    let branches = piece_branches_world(piece);
    if branches.is_empty() {
        return;
    }

    // Special turntable pit rendering
    if piece.connectors.len() >= 6 {
        let center = camera.world_to_screen(rect, piece.transform.x, piece.transform.y);
        let r_world = piece
            .connectors
            .first()
            .map(|c| (c.x - piece.transform.x).hypot(c.y - piece.transform.y))
            .unwrap_or(160.0);
        let r_scr = (r_world as f32) * camera.zoom;
        let pit_fill = if !is_active_layer && !is_selected {
            desaturate_color(Color32::from_rgb(51, 65, 85), 0.8, 0.72)
        } else {
            Color32::from_rgb(51, 65, 85)
        };
        painter.circle_filled(center, r_scr, pit_fill);
        painter.circle_stroke(center, r_scr, Stroke::new(2.0_f32, Color32::from_rgb(148, 163, 184)));
    }

    let mut tie_col = if is_selected {
        Color32::from_rgb(98, 78, 62)
    } else {
        Color32::from_rgb(54, 40, 30)
    };
    let mut rail_col = if is_selected {
        Color32::from_rgb(250, 253, 255)
    } else {
        Color32::from_rgb(220, 228, 238)
    };
    if !is_active_layer && !is_selected {
        tie_col = desaturate_color(tie_col, 0.85, 0.72);
        rail_col = desaturate_color(rail_col, 0.85, 0.68);
    }
    let half_g = gauge * 0.5;
    let half_tie = gauge * 0.72;
    let tie_w = (2.2 * camera.zoom).max(1.0);

    // Pass 1: Selection glow + Ballast roadbed for ALL branches first
    // (prevents a turnout/crossing branch's ballast from covering another branch's ties/rails!)
    let mut screen_branches: Vec<Vec<Pos2>> = Vec::with_capacity(branches.len());
    for pts in &branches {
        if pts.len() < 2 {
            screen_branches.push(Vec::new());
            continue;
        }
        let screen_pts: Vec<Pos2> = pts
            .iter()
            .map(|p| camera.world_to_screen(rect, p.x, p.y))
            .collect();
        if is_selected {
            painter.add(egui::Shape::line(
                screen_pts.clone(),
                Stroke::new(ballast_w + 7.0, Color32::from_rgba_unmultiplied(56, 189, 248, 110)),
            ));
        }
        painter.add(egui::Shape::line(screen_pts.clone(), Stroke::new(ballast_w, ballast_col)));
        screen_branches.push(screen_pts);
    }

    if is_road {
        // Road centerline markings on top of all asphalt branches
        for screen_pts in &screen_branches {
            if screen_pts.len() >= 2 {
                painter.add(egui::Shape::line(
                    screen_pts.clone(),
                    Stroke::new((1.5 * camera.zoom).max(1.0), Color32::from_rgb(241, 245, 249)),
                ));
            }
        }
    } else {
        // Pass 2: Bridge girders + Sleepers / ties across ALL branches
        let tie_step = (gauge / 16.5) * 9.5;
        for pts in &branches {
            if pts.len() < 2 {
                continue;
            }
            if piece.has_bridge == Some(true) {
                let half_br = gauge * 1.15;
                let mut b_left = Vec::with_capacity(pts.len());
                let mut b_right = Vec::with_capacity(pts.len());
                for i in 0..pts.len() {
                    let a = pts[i.saturating_sub(1)];
                    let b = pts[(i + 1).min(pts.len() - 1)];
                    let dx = b.x - a.x;
                    let dy = b.y - a.y;
                    let len = dx.hypot(dy).max(1e-6);
                    let nx = -dy / len;
                    let ny = dx / len;
                    b_left.push(camera.world_to_screen(rect, pts[i].x + nx * half_br, pts[i].y + ny * half_br));
                    b_right.push(camera.world_to_screen(rect, pts[i].x - nx * half_br, pts[i].y - ny * half_br));
                }
                let girder_stroke = Stroke::new((3.0 * camera.zoom).max(1.5), Color32::from_rgb(148, 163, 184));
                painter.add(egui::Shape::line(b_left, girder_stroke));
                painter.add(egui::Shape::line(b_right, girder_stroke));
            }

            let mut accum = 0.0_f64;
            for i in 0..(pts.len() - 1) {
                let a = pts[i];
                let b = pts[i + 1];
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let seg_len = dx.hypot(dy);
                if seg_len < 1e-4 {
                    continue;
                }
                let nx = -dy / seg_len;
                let ny = dx / seg_len;
                let mut d = tie_step - accum;
                while d <= seg_len {
                    let u = d / seg_len;
                    let tx = a.x + dx * u;
                    let ty = a.y + dy * u;
                    let t0 = camera.world_to_screen(rect, tx + nx * half_tie, ty + ny * half_tie);
                    let t1 = camera.world_to_screen(rect, tx - nx * half_tie, ty - ny * half_tie);
                    painter.line_segment([t0, t1], Stroke::new(tie_w, tie_col));
                    d += tie_step;
                }
                accum = seg_len - (d - tie_step);
            }
        }

        // Pass 3: Twin metallic rails on top of ALL ballast & sleepers
        for pts in &branches {
            if pts.len() < 2 {
                continue;
            }
            let mut left_rail = Vec::with_capacity(pts.len());
            let mut right_rail = Vec::with_capacity(pts.len());
            for i in 0..pts.len() {
                let a = pts[i.saturating_sub(1)];
                let b = pts[(i + 1).min(pts.len() - 1)];
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let len = dx.hypot(dy).max(1e-6);
                let nx = -dy / len;
                let ny = dx / len;
                left_rail.push(camera.world_to_screen(rect, pts[i].x + nx * half_g, pts[i].y + ny * half_g));
                right_rail.push(camera.world_to_screen(rect, pts[i].x - nx * half_g, pts[i].y - ny * half_g));
            }
            painter.add(egui::Shape::line(left_rail, Stroke::new(rail_w, rail_col)));
            painter.add(egui::Shape::line(right_rail, Stroke::new(rail_w, rail_col)));
        }
    }

    // Pass 3.5: 2D Tunnel corridor, dashed walls, open-end stone portal brackets & arch badge
    if piece.has_tunnel == Some(true) && piece.connectors.len() < 6 {
        let tunnel_half_w = if is_road {
            road_w_mm * 0.5 + 6.0
        } else {
            (gauge * 1.25).max(20.0)
        };
        let wall_outer_w = (3.0 * camera.zoom).clamp(1.2, 3.2);
        let wall_dash_w = (1.8 * camera.zoom).clamp(0.8, 2.2);
        let corridor_w = (tunnel_half_w as f32 * 2.0 * camera.zoom).max(4.0);
        let wall_outer_col = Color32::from_rgba_unmultiplied(51, 65, 85, 220);
        let wall_dash_col = Color32::from_rgba_unmultiplied(203, 213, 225, 230);

        for pts in &branches {
            if pts.len() < 2 {
                continue;
            }
            let center_scr: Vec<Pos2> = pts
                .iter()
                .map(|p| camera.world_to_screen(rect, p.x, p.y))
                .collect();
            painter.add(egui::Shape::line(
                center_scr,
                Stroke::new(corridor_w, Color32::from_rgba_unmultiplied(15, 23, 42, 76)),
            ));

            let mut left_w = Vec::with_capacity(pts.len());
            let mut right_w = Vec::with_capacity(pts.len());
            for i in 0..pts.len() {
                let a = pts[i.saturating_sub(1)];
                let b = pts[(i + 1).min(pts.len() - 1)];
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let len = dx.hypot(dy).max(1e-6);
                let nx = -dy / len;
                let ny = dx / len;
                left_w.push(Vec2 {
                    x: pts[i].x + nx * tunnel_half_w,
                    y: pts[i].y + ny * tunnel_half_w,
                });
                right_w.push(Vec2 {
                    x: pts[i].x - nx * tunnel_half_w,
                    y: pts[i].y - ny * tunnel_half_w,
                });
            }
            for side in [&left_w, &right_w] {
                let side_scr: Vec<Pos2> = side
                    .iter()
                    .map(|p| camera.world_to_screen(rect, p.x, p.y))
                    .collect();
                painter.add(egui::Shape::line(side_scr, Stroke::new(wall_outer_w, wall_outer_col)));

                // Dashed inner stone wall lining (dash 10 mm / gap 7 mm)
                let dash = 10.0_f64;
                let gap = 7.0_f64;
                let period = dash + gap;
                let mut dist_acc = 0.0_f64;
                for w in side.windows(2) {
                    let seg_len = (w[1].x - w[0].x).hypot(w[1].y - w[0].y);
                    if seg_len < 0.2 {
                        continue;
                    }
                    let mut local_d = 0.0_f64;
                    while local_d < seg_len {
                        let phase = dist_acc % period;
                        if phase < dash {
                            let rem_dash = (dash - phase).min(seg_len - local_d);
                            let u0 = local_d / seg_len;
                            let u1 = (local_d + rem_dash) / seg_len;
                            let a_w = Vec2 {
                                x: w[0].x + (w[1].x - w[0].x) * u0,
                                y: w[0].y + (w[1].y - w[0].y) * u0,
                            };
                            let b_w = Vec2 {
                                x: w[0].x + (w[1].x - w[0].x) * u1,
                                y: w[0].y + (w[1].y - w[0].y) * u1,
                            };
                            painter.line_segment(
                                [
                                    camera.world_to_screen(rect, a_w.x, a_w.y),
                                    camera.world_to_screen(rect, b_w.x, b_w.y),
                                ],
                                Stroke::new(wall_dash_w, wall_dash_col),
                            );
                            local_d += rem_dash;
                            dist_acc += rem_dash;
                        } else {
                            let rem_gap = (period - phase).min(seg_len - local_d);
                            local_d += rem_gap;
                            dist_acc += rem_gap;
                        }
                    }
                }
            }
        }

        let neighbor_has_tunnel = |c_idx: usize| -> bool {
            piece
                .connectors
                .get(c_idx)
                .and_then(|c| c.connected_to.as_ref())
                .and_then(|link| all_pieces.iter().find(|op| op.id == link.piece_id))
                .map_or(false, |op| op.has_tunnel == Some(true))
        };
        let draw_portal_bracket = |px: f64, py: f64, inward_dx: f64, inward_dy: f64| {
            let len = inward_dx.hypot(inward_dy).max(1e-6);
            let ux = inward_dx / len;
            let uy = inward_dy / len;
            let nx = -uy;
            let ny = ux;
            let w = tunnel_half_w + 3.0;
            let wing_len = 10.0;
            let pts_w = [
                Vec2 { x: px + nx * (w + 5.0) - ux * wing_len, y: py + ny * (w + 5.0) - uy * wing_len },
                Vec2 { x: px + nx * w, y: py + ny * w },
                Vec2 { x: px - nx * w, y: py - ny * w },
                Vec2 { x: px - nx * (w + 5.0) - ux * wing_len, y: py - ny * (w + 5.0) - uy * wing_len },
            ];
            let spts: Vec<Pos2> = pts_w
                .iter()
                .map(|p| camera.world_to_screen(rect, p.x, p.y))
                .collect();
            painter.add(egui::Shape::line(
                spts.clone(),
                Stroke::new((4.6 * camera.zoom).clamp(2.2, 5.2), Color32::from_rgb(51, 65, 85)),
            ));
            painter.add(egui::Shape::line(
                spts,
                Stroke::new((2.2 * camera.zoom).clamp(1.2, 2.8), Color32::from_rgb(203, 213, 225)),
            ));
        };

        if let Some(main_b) = branches.first().filter(|b| b.len() >= 2) {
            if !neighbor_has_tunnel(0) {
                let p0 = main_b[0];
                let p1 = main_b[1];
                draw_portal_bracket(p0.x, p0.y, p1.x - p0.x, p1.y - p0.y);
            }
            if piece.connectors.len() >= 2 && !neighbor_has_tunnel(1) {
                let pn = main_b[main_b.len() - 1];
                let pp = main_b[main_b.len() - 2];
                draw_portal_bracket(pn.x, pn.y, pp.x - pn.x, pp.y - pn.y);
            }
            if camera.zoom >= 0.18 {
                let mp = main_b[main_b.len() / 2];
                let sp = camera.world_to_screen(rect, mp.x, mp.y);
                let br = Rect::from_center_size(Pos2::new(sp.x, sp.y + 13.0), EguiVec2::new(16.0, 12.0));
                painter.rect_filled(br, 3.0, Color32::from_rgba_unmultiplied(30, 41, 59, 230));
                painter.rect_stroke(br, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(148, 163, 184)), egui::StrokeKind::Inside);
                painter.text(br.center(), egui::Align2::CENTER_CENTER, "T", FontId::monospace(8.5), Color32::WHITE);
            }
        }
    }

    // Pass 4: Overhead Catenary Wire & Masts
    if piece.has_catenary == Some(true) && !is_road {
        let mut cat_col = if power_sim_enabled {
            match piece_power.and_then(|pr| pr.catenary_status) {
                Some(PowerStatus::Optimal) => Color32::from_rgb(16, 185, 129),
                Some(PowerStatus::Warning) => Color32::from_rgb(245, 158, 11),
                Some(PowerStatus::Critical) => Color32::from_rgb(239, 68, 68),
                Some(PowerStatus::Disconnected) => Color32::from_rgb(220, 38, 38),
                None => Color32::from_rgb(14, 165, 233),
            }
        } else {
            Color32::from_rgb(14, 165, 233)
        };
        if !is_active_layer && !is_selected {
            cat_col = desaturate_color(cat_col, 0.78, 0.72);
        }
        let wire_w = (1.8 * camera.zoom).clamp(1.2, 2.8);
        let mast_col = Color32::from_rgb(51, 65, 85);
        for pts in &branches {
            if pts.len() < 2 {
                continue;
            }
            let spts: Vec<Pos2> = pts
                .iter()
                .map(|p| camera.world_to_screen(rect, p.x, p.y))
                .collect();
            painter.add(egui::Shape::line(spts, Stroke::new(wire_w, cat_col)));

            let mid_i = pts.len() / 2;
            let a = pts[mid_i.saturating_sub(1)];
            let b = pts[(mid_i + 1).min(pts.len() - 1)];
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let len = dx.hypot(dy).max(1e-6);
            let mast_off = (gauge * 1.15).max(20.0);
            let mp = pts[mid_i];
            let mx = mp.x + (-dy / len) * mast_off;
            let my = mp.y + (dx / len) * mast_off;
            let sm = camera.world_to_screen(rect, mx, my);
            let sc = camera.world_to_screen(rect, mp.x, mp.y);
            painter.line_segment([sm, sc], Stroke::new((1.4 * camera.zoom).max(1.0), mast_col));
            painter.circle_filled(sm, (2.4 * camera.zoom).clamp(1.8, 4.0), mast_col);
        }
    }

    // Article number label with dark background pill for high readability
    let main_pts = &branches[0];
    let mid_w = main_pts[main_pts.len() / 2];
    let mid = camera.world_to_screen(rect, mid_w.x, mid_w.y);
    if show_labels {
        // Scale label so the pill is slightly taller than the gauge and never taller than the track roadbed
        let gauge_px = (gauge as f32) * camera.zoom;
        let min_h = (gauge_px * 1.12).max(8.5);
        let max_h = ballast_w.max(min_h);
        let target_h = (gauge_px * 1.25).clamp(min_h, max_h);
        let pad_y = (target_h * 0.12).clamp(1.0, 3.0);
        let font_sz = (target_h - pad_y * 2.0).max(6.5);
        let pad_x = (font_sz * 0.32).clamp(2.5, 6.0);

        let txt_col = if is_active_layer || is_selected {
            Color32::WHITE
        } else {
            Color32::from_rgba_unmultiplied(186, 198, 212, 165)
        };
        let bg_alpha = if is_active_layer || is_selected { 220 } else { 135 };
        let label_text = if power_sim_enabled && !is_road {
            if let Some(pr) = piece_power {
                if pr.is_short_circuit {
                    format!("{} • SHORT!", piece.catalog_id)
                } else if pr.status == PowerStatus::Disconnected {
                    format!("{} • 0V", piece.catalog_id)
                } else {
                    format!("{} • {:.1}V", piece.catalog_id, pr.voltage)
                }
            } else {
                piece.catalog_id.clone()
            }
        } else {
            piece.catalog_id.clone()
        };
        let galley = painter.layout_no_wrap(
            label_text,
            FontId::proportional(font_sz),
            txt_col,
        );
        let pad = EguiVec2::new(pad_x, pad_y);
        let pill_h = (galley.size().y + pad_y * 2.0).clamp(min_h, max_h);
        let pill_rect = Rect::from_center_size(mid, EguiVec2::new(galley.size().x + pad_x * 2.0, pill_h));
        painter.rect_filled(pill_rect, (pill_h * 0.22).clamp(2.0, 5.0), Color32::from_rgba_unmultiplied(15, 23, 42, bg_alpha));
        painter.rect_stroke(
            pill_rect,
            (pill_h * 0.22).clamp(2.0, 5.0),
            Stroke::new(0.8_f32, Color32::from_rgba_unmultiplied(148, 163, 184, if is_active_layer || is_selected { 120 } else { 65 })),
            egui::StrokeKind::Inside,
        );
        let text_pos = Pos2::new(
            pill_rect.center().x - galley.size().x * 0.5,
            pill_rect.center().y - galley.size().y * 0.5,
        );
        let _ = pad;
        painter.galley(text_pos, galley, txt_col);
    }

    // Power feed & isolator indicators
    if piece.power_feed == Some(true) {
        painter.circle_filled(Pos2::new(mid.x, mid.y - 10.0), 4.5, Color32::from_rgb(245, 158, 11));
        painter.circle_stroke(Pos2::new(mid.x, mid.y - 10.0), 5.5, Stroke::new(1.0_f32, Color32::WHITE));
    }
    if piece.is_isolator == Some(true) {
        painter.circle_filled(Pos2::new(mid.x + 10.0, mid.y - 10.0), 4.5, Color32::from_rgb(220, 38, 38));
        painter.circle_stroke(Pos2::new(mid.x + 10.0, mid.y - 10.0), 5.5, Stroke::new(1.0_f32, Color32::WHITE));
    }

    // Catenary power feeder & isolator badges
    if piece.catenary_power_feed == Some(true) && !is_road {
        let p = Pos2::new(mid.x - 13.0, mid.y - 11.0);
        let r = Rect::from_center_size(p, EguiVec2::new(20.0, 12.0));
        painter.rect_filled(r, 4.0, Color32::from_rgb(2, 132, 199));
        painter.rect_stroke(r, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(125, 211, 252)), egui::StrokeKind::Inside);
        painter.text(p, egui::Align2::CENTER_CENTER, "OL+", FontId::proportional(7.5), Color32::WHITE);
    }
    if piece.catenary_isolator == Some(true) && !is_road {
        let c_idx = piece.catenary_isolated_connector_index.unwrap_or(0);
        if let Some(c) = piece.connectors.get(c_idx).or_else(|| piece.connectors.first()) {
            let rad = c.angle.to_radians();
            let in_x = -rad.cos();
            let in_y = -rad.sin();
            let px = -rad.sin();
            let py = rad.cos();
            let cx = c.x + in_x * 8.0;
            let cy = c.y + in_y * 8.0;
            let a = camera.world_to_screen(rect, cx - px * 7.0, cy - py * 7.0);
            let b = camera.world_to_screen(rect, cx + px * 7.0, cy + py * 7.0);
            painter.line_segment([a, b], Stroke::new(3.0_f32, Color32::from_rgb(180, 83, 9)));
            let sc = camera.world_to_screen(rect, cx, cy);
            let badge_p = Pos2::new(sc.x, sc.y - 13.0);
            let br = Rect::from_center_size(badge_p, EguiVec2::new(19.0, 11.0));
            painter.rect_filled(br, 3.0, Color32::from_rgb(3, 105, 161));
            painter.rect_stroke(br, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(239, 68, 68)), egui::StrokeKind::Inside);
            painter.text(badge_p, egui::Align2::CENTER_CENTER, "OL/", FontId::proportional(7.5), Color32::WHITE);
        }
    }

    // Turnout lantern & digital decoder overlays
    let is_left_turnout = kind == "turnout-left" || kind == "curved-turnout-left";
    let acc_side_y = if is_left_turnout { -22.0 } else { 22.0 };
    if piece.has_lantern == Some(true) || piece.has_decoder == Some(true) {
        let root_r = rotate(Vec2 { x: 26.0, y: 0.0 }, piece.transform.rotation);
        let l_r = rotate(Vec2 { x: 26.0, y: acc_side_y }, piece.transform.rotation);
        let s_root = camera.world_to_screen(rect, piece.transform.x + root_r.x, piece.transform.y + root_r.y);
        let s_lant = camera.world_to_screen(rect, piece.transform.x + l_r.x, piece.transform.y + l_r.y);
        painter.line_segment([s_root, s_lant], Stroke::new((1.5 * camera.zoom).max(1.0), Color32::from_rgb(51, 65, 85)));

        if piece.has_lantern == Some(true) {
            let sz = (7.0 * camera.zoom).clamp(5.0, 10.0);
            let r = Rect::from_center_size(s_lant, EguiVec2::splat(sz));
            painter.circle_stroke(s_lant, sz * 0.85, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(254, 240, 138, 130)));
            painter.rect_filled(r, 1.5, Color32::from_rgb(30, 41, 59));
            painter.rect_stroke(r, 1.5, Stroke::new(1.0_f32, Color32::from_rgb(148, 163, 184)), egui::StrokeKind::Inside);
            painter.circle_filled(s_lant, (sz * 0.28).max(1.6), Color32::from_rgb(254, 240, 138));
        }

        if piece.has_decoder == Some(true) {
            let dec_x_local = if piece.has_lantern == Some(true) { 38.0 } else { 26.0 };
            let d_r = rotate(Vec2 { x: dec_x_local, y: acc_side_y }, piece.transform.rotation);
            let s_dec = camera.world_to_screen(rect, piece.transform.x + d_r.x, piece.transform.y + d_r.y);
            let dr = Rect::from_center_size(s_dec, EguiVec2::new(22.0, 11.0));
            painter.rect_filled(dr, 2.5, Color32::from_rgb(15, 23, 42));
            painter.rect_stroke(dr, 2.5, Stroke::new(1.1_f32, Color32::from_rgb(2, 132, 199)), egui::StrokeKind::Inside);
            painter.text(s_dec, egui::Align2::CENTER_CENTER, "DEC", FontId::monospace(7.0), Color32::from_rgb(56, 189, 248));
        }
    }

    // Trackside Signal(s)
    if piece.has_signal == Some(true) && !is_road && main_pts.len() >= 2 {
        let lat_dist = gauge * 0.95 + 7.0;
        let side_cfg = piece.signal_side.as_deref().unwrap_or("right");
        let dir_cfg = piece.signal_direction.as_deref().unwrap_or("forward");
        let sides: &[f64] = match side_cfg {
            "left" => &[-1.0],
            "both" => &[-1.0, 1.0],
            _ => &[1.0],
        };
        let dirs: &[bool] = match dir_cfg {
            "backward" => &[false],
            "both" => &[true, false],
            _ => &[true],
        };
        for &side_f in sides {
            for &is_fwd in dirs {
                let frac = if is_fwd { 0.35 } else { 0.65 };
                let idx = (((main_pts.len() - 1) as f64 * frac).round() as usize).clamp(0, main_pts.len() - 2);
                let a = main_pts[idx];
                let b = main_pts[idx + 1];
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let len = dx.hypot(dy).max(1e-6);
                let ux = dx / len;
                let uy = dy / len;
                let nx = -uy;
                let ny = ux;
                let sx = a.x + nx * lat_dist * side_f;
                let sy = a.y + ny * lat_dist * side_f;
                let s_base = camera.world_to_screen(rect, a.x + nx * (gauge * 0.5) * side_f, a.y + ny * (gauge * 0.5) * side_f);
                let s_sig = camera.world_to_screen(rect, sx, sy);
                painter.line_segment([s_base, s_sig], Stroke::new(1.2_f32, Color32::from_rgb(71, 85, 105)));

                let fwd_sign = if is_fwd { 1.0 } else { -1.0 };
                let red_w = Vec2 { x: sx + ux * 3.5 * fwd_sign, y: sy + uy * 3.5 * fwd_sign };
                let grn_w = Vec2 { x: sx - ux * 3.5 * fwd_sign, y: sy - uy * 3.5 * fwd_sign };
                let s_red = camera.world_to_screen(rect, red_w.x, red_w.y);
                let s_grn = camera.world_to_screen(rect, grn_w.x, grn_w.y);
                painter.line_segment([s_grn, s_red], Stroke::new(8.5_f32, Color32::from_rgb(15, 23, 42)));
                painter.circle_filled(s_red, 2.5, Color32::from_rgb(239, 68, 68));
                painter.circle_filled(s_grn, 2.5, Color32::from_rgb(34, 197, 94));
            }
        }
    }
}

fn piece_branches_world(piece: &TrackPiece) -> Vec<Vec<Vec2>> {
    // 1. Flextrack spline
    if let (Some(offsets), Some(chord)) = (&piece.custom_offsets, piece.custom_chord_dist) {
        if offsets.len() >= 3 {
            let flat = sample_flex_spline_points(chord, offsets[0], offsets[1], offsets[2], 28);
            let mut pts = Vec::with_capacity(29);
            for i in (0..flat.len()).step_by(5) {
                let rp = rotate(Vec2 { x: flat[i], y: flat[i + 1] }, piece.transform.rotation);
                pts.push(Vec2 {
                    x: piece.transform.x + rp.x,
                    y: piece.transform.y + rp.y,
                });
            }
            return vec![pts];
        }
    }

    let c = &piece.connectors;
    match c.len() {
        0 => vec![],
        1 => {
            // Buffer stop (Prellbock): short stub from c[0]
            let rad = (c[0].angle + 180.0).to_radians();
            vec![vec![
                Vec2 { x: c[0].x, y: c[0].y },
                Vec2 { x: c[0].x + rad.cos() * 70.0, y: c[0].y + rad.sin() * 70.0 },
            ]]
        }
        2 => vec![sample_arc_between_connectors(
            c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 18,
        )],
        3 => vec![
            sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 16),
            sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[2].x, c[2].y, c[2].angle, 16),
        ],
        4 => {
            // Distinguish 3-way turnout (1 entry + 3 exits facing away from c0)
            // vs crossing / double-slip (2 opposite pairs).
            let a0 = c[0].angle.to_radians();
            let v0x = a0.cos();
            let v0y = a0.sin();
            let facing_opposite = c[1..4]
                .iter()
                .filter(|ci| {
                    let ai = ci.angle.to_radians();
                    v0x * ai.cos() + v0y * ai.sin() < -0.15
                })
                .count();
            if facing_opposite == 3 {
                // 3-way turnout
                vec![
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 14),
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[2].x, c[2].y, c[2].angle, 14),
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[3].x, c[3].y, c[3].angle, 14),
                ]
            } else {
                // Crossing or double-slip
                vec![
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 14),
                    sample_arc_between_connectors(c[2].x, c[2].y, c[2].angle, c[3].x, c[3].y, c[3].angle, 14),
                ]
            }
        }
        _ => {
            // Turntable: bridge across center
            let mut out = Vec::new();
            let half = c.len() / 2;
            out.push(vec![
                Vec2 { x: c[0].x, y: c[0].y },
                Vec2 { x: c[half].x, y: c[half].y },
            ]);
            out
        }
    }
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

fn is_layer_editable(layout: &Layout, layer_id: Option<&str>) -> bool {
    let lid = layer_id.unwrap_or("layer-1");
    layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == lid))
        .map(|l| l.visible && !l.locked)
        .unwrap_or(true)
}

fn draw_model_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    m: &Model3D,
    is_sel: bool,
    is_active_layer: bool,
    show_labels: bool,
) {
    let scale = m.scale.unwrap_or(1.0);
    let hw = m.width * scale * 0.5;
    let hd = m.depth * scale * 0.5;
    let rot = m.transform.rotation;
    let corners = [
        Vec2 { x: -hw, y: -hd },
        Vec2 { x: hw, y: -hd },
        Vec2 { x: hw, y: hd },
        Vec2 { x: -hw, y: hd },
    ];
    let spts: Vec<Pos2> = corners
        .iter()
        .map(|c| {
            let rc = rotate(*c, rot);
            camera.world_to_screen(rect, m.transform.x + rc.x, m.transform.y + rc.y)
        })
        .collect();

    let is_mtn = m.builtin_id.as_deref() == Some("mountain");
    let is_ground = m.builtin_id.as_deref() == Some("ground-terrain");
    let g_type = m.ground_terrain_type.as_deref().unwrap_or("hill");
    let is_rail_bldg = matches!(
        m.builtin_id.as_deref(),
        Some("engine-shed")
            | Some("turntable-trainhall")
            | Some("coaling-station")
            | Some("diesel-fuel-station")
            | Some("buffer-stop")
            | Some("platform")
            | Some("bahnsteig")
    );
    let is_industrial = matches!(
        m.builtin_id.as_deref(),
        Some("industry-building") | Some("skyscraper")
    );
    let is_custom_glb = m.file_data.is_some() || matches!(m.model_type.as_str(), "glb" | "gltf");

    let sel_stroke = Color32::from_rgb(56, 189, 248);

    if is_ground {
        let (mut outer_col, mut inner_col, mut stroke_col) = match g_type {
            "water" => (
                Color32::from_rgba_unmultiplied(2, 132, 199, 115),
                Color32::from_rgba_unmultiplied(56, 189, 248, 130),
                Color32::from_rgb(3, 105, 161),
            ),
            "sand" => (
                Color32::from_rgba_unmultiplied(214, 185, 124, 132),
                Color32::from_rgba_unmultiplied(231, 211, 163, 145),
                Color32::from_rgb(161, 130, 74),
            ),
            "dirt" => (
                Color32::from_rgba_unmultiplied(107, 76, 48, 132),
                Color32::from_rgba_unmultiplied(133, 96, 62, 145),
                Color32::from_rgb(74, 51, 31),
            ),
            _ => (
                Color32::from_rgba_unmultiplied(77, 124, 15, 122),
                Color32::from_rgba_unmultiplied(101, 163, 13, 135),
                Color32::from_rgb(54, 83, 20),
            ),
        };
        if is_sel {
            stroke_col = sel_stroke;
        } else if !is_active_layer {
            outer_col = desaturate_color(outer_col, 0.78, 0.72);
            inner_col = desaturate_color(inner_col, 0.78, 0.72);
            stroke_col = desaturate_color(stroke_col, 0.78, 0.72);
        }

        if let Some(nodes) = m.mountain_path.as_ref().filter(|p| !p.is_empty()) {
            let avg_r = (nodes.iter().map(|n| n.radius).sum::<f64>() / nodes.len() as f64) * scale;
            let pts_scr: Vec<Pos2> = nodes
                .iter()
                .map(|n| {
                    let rp = rotate(Vec2 { x: n.x * scale, y: n.y * scale }, rot);
                    camera.world_to_screen(rect, m.transform.x + rp.x, m.transform.y + rp.y)
                })
                .collect();
            if pts_scr.len() >= 2 {
                let w_outer = (avg_r as f32 * 2.0 * camera.zoom).max(8.0);
                let w_inner = (avg_r as f32 * 1.3 * camera.zoom).max(4.0);
                painter.add(egui::Shape::line(pts_scr.clone(), Stroke::new(w_outer, outer_col)));
                painter.add(egui::Shape::line(pts_scr, Stroke::new(w_inner, inner_col)));
            } else {
                let r_scr = ((nodes[0].radius * scale) as f32 * camera.zoom).max(8.0);
                painter.circle_filled(pts_scr[0], r_scr, outer_col);
                painter.circle_filled(pts_scr[0], r_scr * 0.65, inner_col);
            }
            if is_sel {
                painter.add(egui::Shape::closed_line(spts, Stroke::new(2.0_f32, sel_stroke)));
            }
        } else {
            let ellipse_pts = |sx: f64, sy: f64| -> Vec<Pos2> {
                let steps = 28usize;
                (0..steps)
                    .map(|k| {
                        let a = (k as f64 / steps as f64) * std::f64::consts::TAU;
                        let rp = rotate(Vec2 { x: hw * sx * a.cos(), y: hd * sy * a.sin() }, rot);
                        camera.world_to_screen(rect, m.transform.x + rp.x, m.transform.y + rp.y)
                    })
                    .collect()
            };
            painter.add(egui::Shape::convex_polygon(
                ellipse_pts(1.0, 1.0),
                outer_col,
                Stroke::new(if is_sel { 2.4_f32 } else { 1.4_f32 }, stroke_col),
            ));
            painter.add(egui::Shape::convex_polygon(
                ellipse_pts(0.68, 0.68),
                inner_col,
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(stroke_col.r(), stroke_col.g(), stroke_col.b(), 115)),
            ));
            if g_type == "water" {
                for ry in [-0.25, 0.0, 0.25] {
                    let p0 = rotate(Vec2 { x: -hw * 0.35, y: hd * ry }, rot);
                    let p1 = rotate(Vec2 { x: hw * 0.35, y: hd * ry }, rot);
                    painter.line_segment(
                        [
                            camera.world_to_screen(rect, m.transform.x + p0.x, m.transform.y + p0.y),
                            camera.world_to_screen(rect, m.transform.x + p1.x, m.transform.y + p1.y),
                        ],
                        Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(224, 242, 254, 145)),
                    );
                }
            }
        }
    } else if is_mtn {
        let style = m.mountain_style.as_deref().unwrap_or("alpine");
        let (mut c_base, mut c_mid, mut c_upper, mut c_peak) = match style {
            "forest" => (
                Color32::from_rgba_unmultiplied(20, 83, 45, 170),
                Color32::from_rgba_unmultiplied(22, 101, 52, 185),
                Color32::from_rgba_unmultiplied(87, 83, 78, 198),
                Color32::from_rgba_unmultiplied(168, 162, 158, 224),
            ),
            "meadow" => (
                Color32::from_rgba_unmultiplied(54, 83, 20, 170),
                Color32::from_rgba_unmultiplied(77, 124, 15, 185),
                Color32::from_rgba_unmultiplied(101, 163, 13, 198),
                Color32::from_rgba_unmultiplied(132, 204, 22, 224),
            ),
            "rocky" => (
                Color32::from_rgba_unmultiplied(41, 37, 36, 174),
                Color32::from_rgba_unmultiplied(68, 64, 60, 185),
                Color32::from_rgba_unmultiplied(120, 113, 108, 200),
                Color32::from_rgba_unmultiplied(214, 211, 209, 226),
            ),
            _ => (
                Color32::from_rgba_unmultiplied(41, 37, 36, 174),
                Color32::from_rgba_unmultiplied(68, 64, 60, 185),
                Color32::from_rgba_unmultiplied(120, 113, 108, 200),
                Color32::from_rgba_unmultiplied(248, 250, 252, 228),
            ),
        };
        let mut border_col = if is_sel { sel_stroke } else { Color32::from_rgb(68, 64, 60) };
        if !is_active_layer && !is_sel {
            c_base = desaturate_color(c_base, 0.78, 0.72);
            c_mid = desaturate_color(c_mid, 0.78, 0.72);
            c_upper = desaturate_color(c_upper, 0.78, 0.72);
            c_peak = desaturate_color(c_peak, 0.78, 0.72);
            border_col = desaturate_color(border_col, 0.78, 0.72);
        }

        if let Some(nodes) = m.mountain_path.as_ref().filter(|p| !p.is_empty()) {
            let avg_r = (nodes.iter().map(|n| n.radius).sum::<f64>() / nodes.len() as f64) * scale;
            let tiers = [(1.0_f32, c_base), (0.68, c_mid), (0.38, c_upper), (0.18, c_peak)];
            if nodes.len() >= 2 {
                let pts_scr: Vec<Pos2> = nodes
                    .iter()
                    .map(|n| {
                        let rp = rotate(Vec2 { x: n.x * scale, y: n.y * scale }, rot);
                        camera.world_to_screen(rect, m.transform.x + rp.x, m.transform.y + rp.y)
                    })
                    .collect();
                for (t_sc, t_col) in tiers {
                    let sw = (avg_r as f32 * 2.0 * t_sc * camera.zoom).max(4.0);
                    painter.add(egui::Shape::line(pts_scr.clone(), Stroke::new(sw, t_col)));
                }
            } else {
                let n0 = &nodes[0];
                let rp = rotate(Vec2 { x: n0.x * scale, y: n0.y * scale }, rot);
                let sp = camera.world_to_screen(rect, m.transform.x + rp.x, m.transform.y + rp.y);
                for (t_sc, t_col) in tiers {
                    let r_scr = ((n0.radius * scale) as f32 * t_sc * camera.zoom).max(4.0);
                    painter.circle_filled(sp, r_scr, t_col);
                }
            }
            if is_sel {
                painter.add(egui::Shape::closed_line(spts, Stroke::new(2.0_f32, sel_stroke)));
            }
        } else {
            let ellipse_pts = |sx: f64, sy: f64| -> Vec<Pos2> {
                let steps = 28usize;
                (0..steps)
                    .map(|k| {
                        let a = (k as f64 / steps as f64) * std::f64::consts::TAU;
                        let rp = rotate(Vec2 { x: hw * sx * a.cos(), y: hd * sy * a.sin() }, rot);
                        camera.world_to_screen(rect, m.transform.x + rp.x, m.transform.y + rp.y)
                    })
                    .collect()
            };
            painter.add(egui::Shape::convex_polygon(
                ellipse_pts(1.0, 1.0),
                c_base,
                Stroke::new(if is_sel { 2.4_f32 } else { 1.5_f32 }, border_col),
            ));
            painter.add(egui::Shape::convex_polygon(ellipse_pts(0.68, 0.68), c_mid, Stroke::NONE));
            painter.add(egui::Shape::convex_polygon(ellipse_pts(0.38, 0.38), c_upper, Stroke::NONE));
            painter.add(egui::Shape::convex_polygon(
                ellipse_pts(0.18, 0.18),
                c_peak,
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(226, 232, 240, 200)),
            ));
            let cx0 = rotate(Vec2 { x: -hw * 0.25, y: 0.0 }, rot);
            let cx1 = rotate(Vec2 { x: hw * 0.25, y: 0.0 }, rot);
            let cy0 = rotate(Vec2 { x: 0.0, y: -hd * 0.22 }, rot);
            let cy1 = rotate(Vec2 { x: 0.0, y: hd * 0.22 }, rot);
            let x_col = Color32::from_rgba_unmultiplied(255, 255, 255, 210);
            painter.line_segment(
                [
                    camera.world_to_screen(rect, m.transform.x + cx0.x, m.transform.y + cx0.y),
                    camera.world_to_screen(rect, m.transform.x + cx1.x, m.transform.y + cx1.y),
                ],
                Stroke::new(1.3_f32, x_col),
            );
            painter.line_segment(
                [
                    camera.world_to_screen(rect, m.transform.x + cy0.x, m.transform.y + cy0.y),
                    camera.world_to_screen(rect, m.transform.x + cy1.x, m.transform.y + cy1.y),
                ],
                Stroke::new(1.3_f32, x_col),
            );
        }
    } else {
        let mut fill = if is_sel {
            Color32::from_rgba_unmultiplied(96, 210, 255, 130)
        } else if is_custom_glb {
            Color32::from_rgba_unmultiplied(99, 102, 241, 130)
        } else if is_rail_bldg {
            Color32::from_rgba_unmultiplied(156, 92, 68, 135)
        } else if is_industrial {
            Color32::from_rgba_unmultiplied(100, 116, 139, 150)
        } else {
            Color32::from_rgba_unmultiplied(148, 163, 184, 145)
        };
        let mut stroke_col = if is_sel {
            Color32::from_rgb(125, 224, 255)
        } else if is_custom_glb {
            Color32::from_rgb(165, 180, 252)
        } else if is_rail_bldg {
            Color32::from_rgb(226, 164, 132)
        } else if is_industrial {
            Color32::from_rgb(226, 232, 240)
        } else {
            Color32::from_rgb(203, 213, 225)
        };
        if !is_active_layer && !is_sel {
            fill = desaturate_color(fill, 0.78, 0.72);
            stroke_col = desaturate_color(stroke_col, 0.78, 0.72);
        }
        let stroke = Stroke::new(if is_sel { 2.2_f32 } else { 1.4_f32 }, stroke_col);
        painter.add(egui::Shape::convex_polygon(spts, fill, stroke));
    }

    if show_labels {
        let center = camera.world_to_screen(rect, m.transform.x, m.transform.y);
        let font_sz = (10.5 * camera.zoom.sqrt()).clamp(8.0, 12.0);
        let txt_col = if is_active_layer || is_sel {
            Color32::WHITE
        } else {
            Color32::from_rgba_unmultiplied(203, 213, 225, 165)
        };
        let galley = painter.layout_no_wrap(m.name.clone(), FontId::proportional(font_sz), txt_col);
        let pad = EguiVec2::new(4.0, 1.5);
        let pill = Rect::from_center_size(center, galley.size() + pad * 2.0);
        painter.rect_filled(
            pill,
            4.0,
            Color32::from_rgba_unmultiplied(15, 23, 42, if is_active_layer || is_sel { 205 } else { 135 }),
        );
        painter.galley(pill.min + pad, galley, txt_col);
    }
}

fn model_contains_point(m: &Model3D, wp: Vec2) -> bool {
    let scale = m.scale.unwrap_or(1.0);
    if matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain"))
        && m.mountain_path.as_ref().map_or(false, |p| !p.is_empty())
    {
        let (lx, lz) = world_to_mountain_local(wp.x, wp.y, m);
        let (norm_r, _) = evaluate_mountain_local(lx, lz, m);
        return norm_r <= 1.0;
    }
    let local = rotate(
        Vec2 {
            x: wp.x - m.transform.x,
            y: wp.y - m.transform.y,
        },
        -m.transform.rotation,
    );
    let hw = (m.width * scale * 0.5).max(15.0);
    let hd = (m.depth * scale * 0.5).max(15.0);
    local.x.abs() <= hw && local.y.abs() <= hd
}

fn find_building_at(layout: &Layout, wp: Vec2) -> Option<String> {
    let models = layout.models.as_ref()?;
    for m in models.iter().rev() {
        if !is_layer_editable(layout, m.layer_id.as_deref()) {
            continue;
        }
        if matches!(m.builtin_id.as_deref(), Some("ground-terrain") | Some("mountain")) {
            continue;
        }
        if model_contains_point(m, wp) {
            return Some(m.id.clone());
        }
    }
    None
}

fn find_terrain_at(layout: &Layout, wp: Vec2) -> Option<String> {
    let models = layout.models.as_ref()?;
    for m in models.iter().rev() {
        if !is_layer_editable(layout, m.layer_id.as_deref()) {
            continue;
        }
        if !matches!(m.builtin_id.as_deref(), Some("ground-terrain") | Some("mountain")) {
            continue;
        }
        if model_contains_point(m, wp) {
            return Some(m.id.clone());
        }
    }
    None
}

fn find_piece_at(layout: &Layout, wp: Vec2, tolerance: f64) -> Option<String> {
    let mut best: Option<(String, f64)> = None;
    for p in &layout.pieces {
        if !is_layer_editable(layout, p.layer_id.as_deref()) {
            continue;
        }
        for branch in piece_branches_world(p) {
            for w in branch.windows(2) {
                let d = point_to_segment_dist(wp, w[0], w[1]);
                if d <= tolerance && best.as_ref().map_or(true, |(_, bd)| d < *bd) {
                    best = Some((p.id.clone(), d));
                }
            }
        }
    }
    best.map(|(id, _)| id)
}

fn point_to_segment_dist(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    let len_sq = vx * vx + vy * vy;
    if len_sq < 1e-6 {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    let t = (((p.x - a.x) * vx + (p.y - a.y) * vy) / len_sq).clamp(0.0, 1.0);
    let px = a.x + t * vx;
    let py = a.y + t * vy;
    (p.x - px).hypot(p.y - py)
}

struct SnapCandidate {
    src_x: f64,
    src_y: f64,
    src_angle: f64,
    dst_x: f64,
    dst_y: f64,
    dst_angle: f64,
    dist: f64,
}

fn find_best_snap_candidate(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    max_dist: f64,
) -> Option<SnapCandidate> {
    let mut best: Option<SnapCandidate> = None;
    for p in &layout.pieces {
        if !selected.contains(&p.id) {
            continue;
        }
        let p_is_road = crate::catalog::is_road_id(catalogs, &p.catalog_id);
        for c1 in &p.connectors {
            for other in &layout.pieces {
                if selected.contains(&other.id) || !is_layer_visible(layout, other.layer_id.as_deref()) {
                    continue;
                }
                // Never snap tracks to roads or roads to tracks!
                if crate::catalog::is_road_id(catalogs, &other.catalog_id) != p_is_road {
                    continue;
                }
                for c2 in &other.connectors {
                    // Treat connectors connected only to a dragged piece as open too
                    let is_open = match &c2.connected_to {
                        None => true,
                        Some(r) => selected.contains(&r.piece_id),
                    };
                    if !is_open {
                        continue;
                    }
                    let d = (c1.x - c2.x).hypot(c1.y - c2.y);
                    if d <= max_dist && best.as_ref().map_or(true, |b| d < b.dist) {
                        best = Some(SnapCandidate {
                            src_x: c1.x,
                            src_y: c1.y,
                            src_angle: c1.angle,
                            dst_x: c2.x,
                            dst_y: c2.y,
                            dst_angle: c2.angle,
                            dist: d,
                        });
                    }
                }
            }
        }
    }
    best
}

pub fn snap_selected_pieces(
    layout: &mut Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    max_dist: f64,
) -> bool {
    let Some(cand) = find_best_snap_candidate(layout, catalogs, selected, max_dist) else {
        return false;
    };

    // Exact rigid-body rotation around (src_x, src_y) + translation to (dst_x, dst_y).
    // Works for 100% of track types (curves, turnouts, flextracks, multi-selection) without catalog lookup.
    let delta_rot = norm_angle_deg(cand.dst_angle + 180.0 - cand.src_angle);
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

fn draw_rulers(painter: &egui::Painter, rect: Rect, camera: &Camera2D, dark_mode: bool) {
    let top_h = 20.0_f32;
    let left_w = 24.0_f32;
    let bg = if dark_mode {
        Color32::from_rgba_unmultiplied(15, 23, 42, 225)
    } else {
        Color32::from_rgba_unmultiplied(241, 245, 249, 235)
    };
    let border = if dark_mode {
        Color32::from_rgb(51, 65, 85)
    } else {
        Color32::from_rgb(203, 213, 225)
    };
    let txt_col = if dark_mode {
        Color32::from_rgb(148, 163, 184)
    } else {
        Color32::from_rgb(71, 85, 105)
    };

    let top_rect = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + top_h));
    let left_rect = Rect::from_min_max(Pos2::new(rect.min.x, rect.min.y + top_h), Pos2::new(rect.min.x + left_w, rect.max.y));
    painter.rect_filled(top_rect, 0.0, bg);
    painter.rect_filled(left_rect, 0.0, bg);
    painter.line_segment(
        [Pos2::new(rect.min.x, rect.min.y + top_h), Pos2::new(rect.max.x, rect.min.y + top_h)],
        Stroke::new(1.0_f32, border),
    );
    painter.line_segment(
        [Pos2::new(rect.min.x + left_w, rect.min.y), Pos2::new(rect.min.x + left_w, rect.max.y)],
        Stroke::new(1.0_f32, border),
    );

    let step_mm: f64 = if camera.zoom > 1.2 {
        50.0
    } else if camera.zoom > 0.45 {
        100.0
    } else if camera.zoom > 0.18 {
        250.0
    } else {
        500.0
    };

    let tl = camera.screen_to_world(rect, rect.min);
    let br = camera.screen_to_world(rect, rect.max);

    let mut x = (tl.x / step_mm).floor() * step_mm;
    while x <= br.x {
        let sx = camera.world_to_screen(rect, x, 0.0).x;
        if sx >= rect.min.x + left_w && sx <= rect.max.x {
            painter.line_segment(
                [Pos2::new(sx, rect.min.y + top_h - 6.0), Pos2::new(sx, rect.min.y + top_h)],
                Stroke::new(1.0_f32, txt_col),
            );
            painter.text(
                Pos2::new(sx + 2.0, rect.min.y + 3.0),
                egui::Align2::LEFT_TOP,
                format!("{:.0}", x / 10.0),
                FontId::proportional(9.0),
                txt_col,
            );
        }
        x += step_mm;
    }

    let mut y = (tl.y / step_mm).floor() * step_mm;
    while y <= br.y {
        let sy = camera.world_to_screen(rect, 0.0, y).y;
        if sy >= rect.min.y + top_h && sy <= rect.max.y {
            painter.line_segment(
                [Pos2::new(rect.min.x + left_w - 6.0, sy), Pos2::new(rect.min.x + left_w, sy)],
                Stroke::new(1.0_f32, txt_col),
            );
            painter.text(
                Pos2::new(rect.min.x + 2.0, sy + 2.0),
                egui::Align2::LEFT_TOP,
                format!("{:.0}", y / 10.0),
                FontId::proportional(8.5),
                txt_col,
            );
        }
        y += step_mm;
    }

    // Corner unit label (cm)
    painter.rect_filled(Rect::from_min_max(rect.min, Pos2::new(rect.min.x + left_w, rect.min.y + top_h)), 0.0, bg);
    painter.text(
        Pos2::new(rect.min.x + left_w * 0.5, rect.min.y + top_h * 0.5),
        egui::Align2::CENTER_CENTER,
        "cm",
        FontId::proportional(9.0),
        txt_col,
    );
}

fn brighten_color(c: Color32, amount: f32) -> Color32 {
    let t = amount.clamp(0.0, 1.0);
    let r = (c.r() as f32 + (255.0 - c.r() as f32) * t).round() as u8;
    let g = (c.g() as f32 + (255.0 - c.g() as f32) * t).round() as u8;
    let b = (c.b() as f32 + (255.0 - c.b() as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(r, g, b, c.a())
}

fn desaturate_color(c: Color32, sat_drop: f32, alpha_scale: f32) -> Color32 {
    let luma = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    let t = sat_drop.clamp(0.0, 1.0);
    let r = (c.r() as f32 * (1.0 - t) + luma * t).round() as u8;
    let g = (c.g() as f32 * (1.0 - t) + luma * t).round() as u8;
    let b = (c.b() as f32 * (1.0 - t) + luma * t).round() as u8;
    let a = ((c.a() as f32) * alpha_scale.clamp(0.1, 1.0)).round() as u8;
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub fn parse_hex_color(hex: Option<&str>) -> Option<Color32> {
    let s = hex?.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color32::from_rgb(r, g, b))
}
