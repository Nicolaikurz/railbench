use std::collections::HashSet;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2 as EguiVec2};
use railbench_core::geometry::{
    norm_angle_deg, rotate, sample_arc_between_connectors, sample_flex_spline_points,
};
use railbench_core::types::{Catalog, CatalogEntry, Layout, TrackPiece, Vec2};

use crate::catalog::find_entry;

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
        let mut boards = Vec::new();
        if let Some(list) = &layout.baseboards {
            boards.extend(list.iter().filter(|b| b.enabled));
        } else if let Some(bb) = &layout.baseboard {
            if bb.enabled {
                boards.push(bb);
            }
        }
        for bb in boards {
            let (x0, y0) = match bb.position_mode.as_str() {
                "center" => (-bb.width * 0.5, -bb.depth * 0.5),
                "top-left" => (0.0, 0.0),
                _ => (bb.custom_x.unwrap_or(0.0), bb.custom_y.unwrap_or(0.0)),
            };
            min_x = min_x.min(x0);
            max_x = max_x.max(x0 + bb.width);
            min_y = min_y.min(y0);
            max_y = max_y.max(y0 + bb.depth);
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
    pub rotate_delta: Option<f64>,
    pub toggle_feeder: bool,
    pub toggle_isolator: bool,
    pub toggle_catenary: bool,
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
    measure_mode: bool,
    rotation_step: f64,
) -> CanvasInteraction {
    let mut interaction = CanvasInteraction::default();

    let avail = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    // Background (matches variables.css --canvas-bg)
    let bg = crate::theme::palette(dark_mode).canvas_bg;
    painter.rect_filled(rect, 0.0, bg);

    // Zoom with scroll wheel (cursor-anchored)
    if response.hovered() {
        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_y.abs() > 0.01 {
            let factor = (1.0 + scroll_y * 0.0018).clamp(0.75, 1.35);
            if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                let before = camera.screen_to_world(rect, pointer);
                camera.zoom = (camera.zoom * factor).clamp(0.04, 15.0);
                camera.offset.x = pointer.x - rect.min.x - (before.x as f32) * camera.zoom;
                camera.offset.y = pointer.y - rect.min.y - (before.y as f32) * camera.zoom;
            }
        }
    }

    // Pan with middle or secondary mouse button
    if response.dragged_by(egui::PointerButton::Middle)
        || response.dragged_by(egui::PointerButton::Secondary)
    {
        camera.offset += response.drag_delta();
    }

    // 1. Millimeter Grid
    draw_grid(&painter, rect, camera, dark_mode);

    // 2. Baseboards
    draw_baseboards(&painter, rect, camera, layout);

    let active_layer_id = layout.active_layer_id.as_deref().unwrap_or("layer-1");
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

    // Render layer-by-layer from bottom to top so upper layers cleanly cover lower layers
    for (lid, _lh, lcolor) in &visible_layers {
        let is_active_layer = lid.as_str() == active_layer_id;

        // 2.5. 3D Models on this layer
        if let Some(models) = &layout.models {
            for m in models {
                let m_lid = m.layer_id.as_deref().unwrap_or("layer-1");
                if m_lid != lid.as_str() {
                    continue;
                }
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
                let is_sel = selected.contains(&m.id);
                let is_mtn = m.builtin_id.as_deref() == Some("mountain");
                let mut fill = if is_sel {
                    Color32::from_rgba_unmultiplied(96, 210, 255, 110)
                } else if is_mtn {
                    Color32::from_rgba_unmultiplied(34, 197, 94, 52)
                } else {
                    Color32::from_rgba_unmultiplied(148, 163, 184, 70)
                };
                let mut stroke_col = if is_sel {
                    Color32::from_rgb(125, 224, 255)
                } else if is_mtn {
                    Color32::from_rgb(34, 197, 94)
                } else {
                    Color32::from_rgb(148, 163, 184)
                };
                if !is_active_layer && !is_sel {
                    fill = desaturate_color(fill, 0.78, 0.72);
                    stroke_col = desaturate_color(stroke_col, 0.78, 0.72);
                }
                let stroke = Stroke::new(if is_sel { 2.2_f32 } else { 1.2_f32 }, stroke_col);
                painter.add(egui::Shape::convex_polygon(spts, fill, stroke));

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
            draw_track_piece(
                &painter,
                rect,
                camera,
                piece,
                entry,
                is_sel,
                is_active_layer,
                color_mode,
                lcolor.as_deref(),
                show_labels,
                power_sim_enabled,
            );
        }
    }
    let _ = layer_height_of;

    // 4. Connectors (snap points) — skip hidden layers
    if show_connectors {
        for piece in &layout.pieces {
            if !is_layer_visible(layout, piece.layer_id.as_deref()) {
                continue;
            }
            for conn in &piece.connectors {
                let sp = camera.world_to_screen(rect, conn.x, conn.y);
                if !rect.expand(12.0).contains(sp) {
                    continue;
                }
                let r = (4.0 * camera.zoom).clamp(2.5, 6.5);
                if conn.connected_to.is_some() {
                    painter.circle_filled(sp, r * 0.75, Color32::from_rgb(100, 116, 139));
                } else {
                    painter.circle_filled(sp, r, Color32::from_rgb(22, 163, 74));
                    painter.circle_stroke(sp, r + 1.5, Stroke::new(1.0_f32, Color32::from_rgb(34, 197, 94)));
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

    // 5. Primary click selection (tracks OR 3D models; skip when measure tool active)
    if !measure_mode && response.clicked_by(egui::PointerButton::Primary) {
        if let Some(sp) = response.interact_pointer_pos() {
            let wp = camera.screen_to_world(rect, sp);
            let hit = find_piece_at(layout, wp, 22.0 / (camera.zoom as f64).max(0.25))
                .or_else(|| find_model_at(layout, wp));
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
            }
        }
    }

    // 6. Primary drag: Flex handle (3 bend points + endpoint) OR Measure tool OR drag tracks/models OR marquee
    if response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(sp) = response.interact_pointer_pos() {
            let wp = camera.screen_to_world(rect, sp);
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
                if measure_mode {
                    *measure_points = Some((wp, wp));
                } else if let Some(hit) = find_piece_at(layout, wp, 22.0 / (camera.zoom as f64).max(0.25))
                    .or_else(|| find_model_at(layout, wp))
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
                } else {
                    *marquee_start = Some(sp);
                }
            }
        }
    }

    if let Some((ref fid, handle_idx)) = dragging_flex_handle.clone() {
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
            if let Some(cand) = find_best_snap_candidate(layout, catalogs, selected, snap_radius) {
                let s1 = camera.world_to_screen(rect, cand.src_x, cand.src_y);
                let s2 = camera.world_to_screen(rect, cand.dst_x, cand.dst_y);
                let cyan = Color32::from_rgb(56, 189, 248);
                painter.line_segment([s1, s2], Stroke::new(1.8_f32, cyan));
                painter.circle_stroke(s2, 10.0, Stroke::new(2.2_f32, cyan));
                painter.circle_filled(s2, 4.0, Color32::from_rgb(34, 197, 94));
            }
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
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
        if *dragging_active {
            *dragging_active = false;
            *drag_last_world = None;

            // Magnetic rigid-body snap for single OR multi-piece selection
            if snapping_enabled && !selected.is_empty() {
                let snap_radius = (42.0 / (camera.zoom as f64).max(0.2)).clamp(35.0, 95.0);
                if snap_selected_pieces(layout, catalogs, selected, snap_radius) {
                    interaction.changed = true;
                }
            }
            railbench_core::snapping::update_layout_connections(layout, 4.0, 35.0, 1.0);
            crate::catalog::disconnect_cross_type_connections(layout, catalogs);
            interaction.changed = true;
        }
    }

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

    // 9. Right-click track context menu (matches src/ui/track-context-menu.ts)
    if !selected.is_empty() {
        response.context_menu(|ui| {
            if ui.button(format!("↺ Rotate -{rotation_step}°")).clicked() {
                interaction.rotate_delta = Some(-rotation_step);
                ui.close_menu();
            }
            if ui.button(format!("↻ Rotate +{rotation_step}°")).clicked() {
                interaction.rotate_delta = Some(rotation_step);
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
            if ui.button("Toggle Bridge (B)").clicked() {
                interaction.toggle_bridge = true;
                ui.close_menu();
            }
            if ui.button("Toggle Tunnel").clicked() {
                interaction.toggle_tunnel = true;
                ui.close_menu();
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

fn draw_baseboards(painter: &egui::Painter, rect: Rect, camera: &Camera2D, layout: &Layout) {
    let mut boards = Vec::new();
    if let Some(list) = &layout.baseboards {
        boards.extend(list.iter().filter(|b| b.enabled));
    } else if let Some(b) = &layout.baseboard {
        if b.enabled {
            boards.push(b);
        }
    }
    for b in boards {
        // Match TS getBaseboardBounds(): positionMode "center" => (-W/2,-D/2), "top-left" => (0,0), "custom" => (customX, customY) as TOP-LEFT!
        let (x0, y0) = match b.position_mode.as_str() {
            "center" => (-b.width * 0.5, -b.depth * 0.5),
            "top-left" => (0.0, 0.0),
            _ => (b.custom_x.unwrap_or(0.0), b.custom_y.unwrap_or(0.0)),
        };
        let fill = match b.style.as_str() {
            "grass" => Color32::from_rgb(54, 83, 20),
            "dark-wood" => Color32::from_rgb(78, 52, 36),
            "slate" => Color32::from_rgb(51, 65, 85),
            "white" => Color32::from_rgb(241, 245, 249),
            _ => Color32::from_rgb(180, 142, 98),
        };
        let border_stroke = Stroke::new(2.0_f32, Color32::from_rgb(30, 41, 59));

        let rot = b.rotation.unwrap_or(0.0);
        if let Some(poly) = &b.custom_polygon {
            if poly.len() >= 3 {
                let spts: Vec<Pos2> = poly
                    .iter()
                    .map(|p| camera.world_to_screen(rect, x0 + p.x, y0 + p.y))
                    .collect();
                painter.add(egui::Shape::convex_polygon(spts, fill, border_stroke));
                continue;
            }
        }

        if rot.abs() > 1e-3 {
            let cx = x0 + b.width * 0.5;
            let cy = y0 + b.depth * 0.5;
            let corners = [
                Vec2 { x: -b.width * 0.5, y: -b.depth * 0.5 },
                Vec2 { x: b.width * 0.5, y: -b.depth * 0.5 },
                Vec2 { x: b.width * 0.5, y: b.depth * 0.5 },
                Vec2 { x: -b.width * 0.5, y: b.depth * 0.5 },
            ];
            let spts: Vec<Pos2> = corners
                .iter()
                .map(|c| {
                    let rc = rotate(*c, rot);
                    camera.world_to_screen(rect, cx + rc.x, cy + rc.y)
                })
                .collect();
            painter.add(egui::Shape::convex_polygon(spts, fill, border_stroke));
        } else {
            let p0 = camera.world_to_screen(rect, x0, y0);
            let p1 = camera.world_to_screen(rect, x0 + b.width, y0 + b.depth);
            let r = Rect::from_two_pos(p0, p1);
            painter.rect_filled(r, 4.0, fill);
            painter.rect_stroke(r, 4.0, border_stroke, egui::StrokeKind::Outside);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_track_piece(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    is_selected: bool,
    is_active_layer: bool,
    color_mode: ColorMode,
    layer_color_hex: Option<&str>,
    show_labels: bool,
    power_sim_enabled: bool,
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
        if piece.is_isolator == Some(true) {
            Color32::from_rgb(220, 38, 38)
        } else if piece.power_feed == Some(true) {
            Color32::from_rgb(22, 163, 74)
        } else {
            Color32::from_rgb(34, 197, 94)
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
        let galley = painter.layout_no_wrap(
            piece.catalog_id.clone(),
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

fn find_model_at(layout: &Layout, wp: Vec2) -> Option<String> {
    let models = layout.models.as_ref()?;
    // Iterate in reverse so topmost models win
    for m in models.iter().rev() {
        if !is_layer_editable(layout, m.layer_id.as_deref()) {
            continue;
        }
        let scale = m.scale.unwrap_or(1.0);
        let hw = (m.width * scale * 0.5).max(15.0);
        let hd = (m.depth * scale * 0.5).max(15.0);
        let local = rotate(
            Vec2 {
                x: wp.x - m.transform.x,
                y: wp.y - m.transform.y,
            },
            -m.transform.rotation,
        );
        if local.x.abs() <= hw && local.y.abs() <= hd {
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
