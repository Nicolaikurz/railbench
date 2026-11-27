use super::*;

pub(super) fn snap_measure_point(raw: Vec2, layout: &Layout, alt_held: bool, zoom: f32) -> (Vec2, bool) {
    if !alt_held {
        return (raw, false);
    }
    let thresh = (28.0 / zoom.max(0.25) as f64).clamp(12.0, 90.0);
    let mut best_d2 = thresh * thresh;
    let mut best_pt = raw;
    let mut snapped = false;
    for p in &layout.pieces {
        if !is_layer_visible(layout, p.layer_id.as_deref()) {
            continue;
        }
        for c in &p.connectors {
            let d2 = (c.x - raw.x).powi(2) + (c.y - raw.y).powi(2);
            if d2 < best_d2 {
                best_d2 = d2;
                best_pt = Vec2 { x: c.x, y: c.y };
                snapped = true;
            }
        }
        let d2 = (p.transform.x - raw.x).powi(2) + (p.transform.y - raw.y).powi(2);
        if d2 < best_d2 {
            best_d2 = d2;
            best_pt = Vec2 {
                x: p.transform.x,
                y: p.transform.y,
            };
            snapped = true;
        }
    }
    if let Some(models) = &layout.models {
        for m in models {
            if !is_layer_visible(layout, m.layer_id.as_deref()) {
                continue;
            }
            let d2 = (m.transform.x - raw.x).powi(2) + (m.transform.y - raw.y).powi(2);
            if d2 < best_d2 {
                best_d2 = d2;
                best_pt = Vec2 {
                    x: m.transform.x,
                    y: m.transform.y,
                };
                snapped = true;
            }
        }
    }
    (best_pt, snapped)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_canvas_interaction(
    ui: &mut egui::Ui,
    rect: Rect,
    response: &egui::Response,
    painter: &egui::Painter,
    hover_pos: Option<Pos2>,
    layout: &mut Layout,
    cache: &Canvas2DCache,
    catalogs: &[Catalog],
    camera: &Camera2D,
    selected: &mut HashSet<String>,
    dragging_active: &mut bool,
    drag_last_world: &mut Option<Vec2>,
    dragging_flex_handle: &mut Option<(String, usize)>,
    marquee_start: &mut Option<Pos2>,
    measure_points: &mut Option<(Vec2, Vec2)>,
    measure_locked: &mut bool,
    snapping_enabled: bool,
    layer_protection: bool,
    board_edit_enabled: bool,
    selection_scope: crate::SelectionScope,
    sidebar_drag_active: bool,
    measure_mode: bool,
    terrain_brush: &mut TerrainBrushState,
    is_de: bool,
    next_piece_seq: &mut u64,
    active_board_id: &mut Option<String>,
    dragging_board: &mut Option<(String, Vec2, f64, f64)>,
    board_snap_guides: &mut Vec<BaseboardSnapGuide>,
    sketcher: &mut BoardSketcherState,
    dragging_model_rot: &mut Option<String>,
    dragging_track_rot: &mut Option<TrackRotationDrag>,
    mut ramp_overlay: Option<HeightPanelOverlay<'_>>,
    active_layer_id: &str,
    board_header_rects: &[(String, Rect)],
    plus_buttons: &[(String, usize, Pos2, bool)],
    accessory_buttons: &[TrackAccessoryButton],
    selected_flex_handles: Option<(String, [Pos2; 5])>,
    selected_model_rot_handle: Option<(String, Pos2)>,
    selected_track_rot_handle: Option<(Vec2, Pos2, f64, Option<String>, f64)>,
    rotation_step: f64,
    interaction: &mut CanvasInteraction,
) {
    interaction.hover_world = hover_pos
        .filter(|sp| rect.contains(*sp))
        .map(|sp| camera.screen_to_world(rect, sp));

    if sidebar_drag_active {
        *dragging_active = false;
        *drag_last_world = None;
        *marquee_start = None;
        *dragging_flex_handle = None;
        *dragging_model_rot = None;
        *dragging_track_rot = None;
        *dragging_board = None;
        board_snap_guides.clear();
        return;
    }

    // Tables/baseboards may only be selected or moved while the Board tab is open
    if !board_edit_enabled {
        if dragging_board.is_some() {
            *dragging_board = None;
            board_snap_guides.clear();
        }
        if sketcher.active {
            sketcher.stop();
        }
    }

    // Cursor feedback when hovering/dragging boards inside the Board tab
    if dragging_board.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if board_edit_enabled
        && !sketcher.active
        && !measure_mode
        && !terrain_brush.active
    {
        if let (Some(hp), Some(hw)) = (hover_pos.filter(|sp| rect.contains(*sp)), interaction.hover_world) {
            let hits_track_or_model = if selection_scope == crate::SelectionScope::Boards {
                false
            } else {
                find_canvas_object_at(
                    layout,
                    cache,
                    catalogs,
                    hw,
                    22.0 / (camera.zoom as f64).max(0.25),
                    layer_protection,
                    selection_scope,
                )
                .is_some()
            };
            if !hits_track_or_model
                && find_board_drag_hit(
                    true,
                    layout,
                    hw,
                    hp,
                    camera.zoom,
                    board_header_rects,
                    true,
                )
                .is_some()
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        }
    }

    // Live hover preview while placing Measure Point B (click-A-then-click-B mode)
    if measure_mode {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        let alt_held = ui.input(|i| i.modifiers.alt);
        if let Some(hw) = interaction.hover_world {
            let (snapped, did_snap) = snap_measure_point(hw, layout, alt_held, camera.zoom);
            if did_snap {
                let s = camera.world_to_screen(rect, snapped.x, snapped.y);
                painter.circle_stroke(s, 8.0, Stroke::new(1.8_f32, Color32::from_rgb(245, 158, 11)));
                painter.circle_filled(s, 3.0, Color32::from_rgb(245, 158, 11));
            }
            if !*measure_locked {
                if let Some((a, _)) = *measure_points {
                    if response.hovered() && !response.dragged_by(egui::PointerButton::Primary) {
                        *measure_points = Some((a, snapped));
                    }
                }
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
    }


    let ramp_pick_active = ramp_overlay.as_ref().is_some_and(|ov| {
        (ov.tab == 1 && ov.ramp_sel.pick_mode.is_some()) || (ov.tab == 2 && *ov.gen_pick_mode)
    });
    primary_click::handle_ramp_and_primary_click(
        ui,
        rect,
        response,
        painter,
        layout,
        cache,
        catalogs,
        camera,
        selected,
        measure_points,
        measure_locked,
        layer_protection,
        board_edit_enabled,
        selection_scope,
        measure_mode,
        terrain_brush,
        is_de,
        active_board_id,
        sketcher,
        &mut ramp_overlay,
        ramp_pick_active,
        board_header_rects,
        plus_buttons,
        accessory_buttons,
        selected_model_rot_handle.as_ref(),
        selected_track_rot_handle.as_ref(),
        interaction,
    );

    if !sketcher.active && !ramp_pick_active {
        primary_drag::handle_primary_drag(
            ui,
            rect,
            response,
            painter,
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
            measure_locked,
            snapping_enabled,
            layer_protection,
            board_edit_enabled,
            selection_scope,
            measure_mode,
            terrain_brush,
            is_de,
            next_piece_seq,
            active_board_id,
            dragging_board,
            board_snap_guides,
            dragging_model_rot,
            dragging_track_rot,
            active_layer_id,
            board_header_rects,
            plus_buttons,
            accessory_buttons,
            selected_flex_handles,
            selected_model_rot_handle,
            selected_track_rot_handle,
            rotation_step,
            interaction,
        );
    }
}

mod primary_click;
mod primary_drag;
