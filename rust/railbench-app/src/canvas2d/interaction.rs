use super::*;

fn snap_measure_point(raw: Vec2, layout: &Layout, alt_held: bool, zoom: f32) -> (Vec2, bool) {
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
    mut ramp_overlay: Option<(&mut crate::ramp_tools::RampSelectionState, f64, f64)>,
    active_layer_id: &str,
    board_header_rects: &[(String, Rect)],
    plus_buttons: &[(String, usize, Pos2, bool)],
    selected_flex_handles: Option<(String, [Pos2; 5])>,
    selected_model_rot_handle: Option<(String, Pos2)>,
    selected_track_rot_handle: Option<(Vec2, Pos2, f64, Option<String>, f64)>,
    rotation_step: f64,
    interaction: &mut CanvasInteraction,
) {
    interaction.hover_world = hover_pos
        .filter(|sp| rect.contains(*sp))
        .map(|sp| camera.screen_to_world(rect, sp));

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

    // 4.7. Live Auto-Ramp Start (A), End (B) & Direction Arrow Overlay
    let ramp_pick_active = ramp_overlay.as_ref().is_some_and(|(rs, _, _)| rs.pick_mode.is_some());
    if let Some((ref ramp_sel, start_h, end_h)) = ramp_overlay {
        let chain = crate::ramp_tools::resolve_ramp_chain(
            layout,
            selected,
            ramp_sel.start_piece.as_deref(),
            ramp_sel.end_piece.as_deref(),
        );
        let segs = crate::ramp_tools::ramp_chain_direction_segments(layout, &chain);

        // Draw glowing ramp path + directional arrowheads along every piece in the chain
        let arrow_col = Color32::from_rgb(56, 189, 248);
        for &(p_in, p_out) in &segs {
            let s_in = camera.world_to_screen(rect, p_in.x, p_in.y);
            let s_out = camera.world_to_screen(rect, p_out.x, p_out.y);
            painter.line_segment(
                [s_in, s_out],
                Stroke::new(3.5_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 175)),
            );
            let dir = s_out - s_in;
            let len = dir.length();
            if len > 14.0 {
                let u = dir / len;
                let perp = egui::vec2(-u.y, u.x);
                let mid = s_in + dir * 0.55;
                let tip = mid + u * 8.0;
                let left = mid - u * 5.0 + perp * 5.5;
                let right = mid - u * 5.0 - perp * 5.5;
                painter.add(egui::Shape::convex_polygon(
                    vec![tip, left, right],
                    arrow_col,
                    Stroke::new(1.0, Color32::from_rgb(15, 23, 42)),
                ));
            }
        }

        // Helper to draw Start (A) or End (B) pin badge
        let draw_ramp_pin = |pos_w: Vec2, letter: &str, label: String, fill: Color32| {
            let sp = camera.world_to_screen(rect, pos_w.x, pos_w.y);
            painter.circle_filled(sp, 11.0, fill);
            painter.circle_stroke(sp, 12.2, Stroke::new(2.0_f32, Color32::WHITE));
            painter.text(
                sp,
                egui::Align2::CENTER_CENTER,
                letter,
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
            let pill = Rect::from_min_size(sp + egui::vec2(14.0, -11.0), egui::vec2(78.0, 22.0));
            painter.rect_filled(pill, 6.0, Color32::from_rgba_unmultiplied(15, 23, 42, 230));
            painter.rect_stroke(pill, 6.0, Stroke::new(1.2_f32, fill), egui::StrokeKind::Inside);
            painter.text(
                pill.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );
        };

        let start_pos_w = segs.first().map(|(a, _)| *a).or_else(|| {
            ramp_sel.start_piece.as_deref().and_then(|sid| {
                layout.pieces.iter().find(|p| p.id == sid).map(|p| {
                    p.connectors
                        .first()
                        .map(|c| Vec2 { x: c.x, y: c.y })
                        .unwrap_or(Vec2 { x: p.transform.x, y: p.transform.y })
                })
            })
        });
        let end_pos_w = segs.last().map(|(_, b)| *b).or_else(|| {
            ramp_sel.end_piece.as_deref().and_then(|eid| {
                layout.pieces.iter().find(|p| p.id == eid).map(|p| {
                    p.connectors
                        .last()
                        .map(|c| Vec2 { x: c.x, y: c.y })
                        .unwrap_or(Vec2 { x: p.transform.x, y: p.transform.y })
                })
            })
        });

        if let Some(a_w) = start_pos_w {
            draw_ramp_pin(a_w, "A", format!("{start_h:.0} mm"), Color32::from_rgb(16, 185, 129));
        }
        if let Some(b_w) = end_pos_w {
            draw_ramp_pin(b_w, "B", format!("{end_h:.0} mm"), Color32::from_rgb(245, 158, 11));
        }

        // Top-center banner + crosshair cursor while picking A or B
        if let Some(picking_start) = ramp_sel.pick_mode {
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
            let (col, msg) = if picking_start {
                (
                    Color32::from_rgb(16, 185, 129),
                    if is_de {
                        "Rampe Startpunkt (A): Klicke ein Gleis (Esc = Abbrechen)"
                    } else {
                        "Ramp Start (A): Click a track piece on the canvas (Esc to cancel)"
                    },
                )
            } else {
                (
                    Color32::from_rgb(245, 158, 11),
                    if is_de {
                        "Rampe Endpunkt (B): Klicke ein Gleis (Esc = Abbrechen)"
                    } else {
                        "Ramp End (B): Click a track piece on the canvas (Esc to cancel)"
                    },
                )
            };
            let banner = Rect::from_center_size(
                Pos2::new(rect.center().x, rect.min.y + 40.0),
                egui::vec2(410.0, 28.0),
            );
            painter.rect_filled(banner, 8.0, Color32::from_rgba_unmultiplied(15, 23, 42, 235));
            painter.rect_stroke(banner, 8.0, Stroke::new(1.5_f32, col), egui::StrokeKind::Inside);
            painter.text(
                banner.center(),
                egui::Align2::CENTER_CENTER,
                msg,
                egui::FontId::proportional(12.0),
                Color32::WHITE,
            );
        }
    }

    if !sketcher.active && ramp_pick_active && response.clicked_by(egui::PointerButton::Primary) {
        if let Some(sp) = response.interact_pointer_pos() {
            let wp = camera.screen_to_world(rect, sp);
            if let Some(pid) = find_piece_at(
                layout,
                cache,
                wp,
                24.0 / (camera.zoom as f64).max(0.25),
                false,
            ) {
                if let Some((ref mut ramp_sel, _, _)) = ramp_overlay {
                    match ramp_sel.pick_mode {
                        Some(true) => {
                            ramp_sel.start_piece = Some(pid);
                            // Automatically advance to picking End (B) if not set yet
                            ramp_sel.pick_mode = if ramp_sel.end_piece.is_none() {
                                Some(false)
                            } else {
                                None
                            };
                        }
                        Some(false) => {
                            ramp_sel.end_piece = Some(pid);
                            ramp_sel.pick_mode = None;
                        }
                        None => {}
                    }
                }
            }
        }
    } else if measure_mode && !ramp_pick_active && response.clicked_by(egui::PointerButton::Primary) {
        if let Some(sp) = response.interact_pointer_pos() {
            let alt_held = ui.input(|i| i.modifiers.alt);
            let raw = camera.screen_to_world(rect, sp);
            let (pt, _) = snap_measure_point(raw, layout, alt_held, camera.zoom);
            if measure_points.is_none() || *measure_locked {
                *measure_points = Some((pt, pt));
                *measure_locked = false;
            } else if let Some((a, _)) = *measure_points {
                *measure_points = Some((a, pt));
                *measure_locked = true;
            }
        }
    } else if terrain_brush.active
        && terrain_brush.is_heightfield_mode()
        && !ramp_pick_active
        && response.clicked_by(egui::PointerButton::Primary)
    {
        if let Some(sp) = response.interact_pointer_pos() {
            let wp = camera.screen_to_world(rect, sp);
            let shift_s = ui.input(|i| i.modifiers.shift);
            let alt_i = ui.input(|i| i.modifiers.alt);
            terrain_brush.flatten_target_mm = None;
            terrain_brush.last_sculpt_world = None;
            interaction.push_undo = true;
            if terrain_brush.apply_live_sculpt_step(wp, layout, shift_s, alt_i) {
                interaction.changed = true;
            }
            terrain_brush.flatten_target_mm = None;
            terrain_brush.last_sculpt_world = None;
            if terrain_brush.auto_tunnel
                && crate::terrain_tools::apply_auto_tunnel_for_heightfield(layout) > 0
            {
                interaction.changed = true;
            }
        }
    } else if !terrain_brush.active && !measure_mode && !ramp_pick_active && response.clicked_by(egui::PointerButton::Primary) {
        if let Some(sp) = response.interact_pointer_pos() {
            let on_rot_knob = selected_track_rot_handle
                .as_ref()
                .is_some_and(|(_, hs, _, _, _)| sp.distance(*hs) <= 14.0)
                || selected_model_rot_handle
                    .as_ref()
                    .is_some_and(|(_, hs)| sp.distance(*hs) <= 14.0);
            if on_rot_knob {
                // Keep selection intact when clicking directly on the rotation handle
            } else if let Some((pid, cidx, _, _)) = plus_buttons
                .iter()
                .find(|(_, _, ppos, _)| sp.distance(*ppos) <= 13.5)
            {
                interaction.quick_add_at = Some((pid.clone(), *cidx));
            } else {
                let wp = camera.screen_to_world(rect, sp);
                let hit = find_building_at(layout, wp, layer_protection)
                    .or_else(|| {
                        find_piece_at(
                            layout,
                            cache,
                            wp,
                            22.0 / (camera.zoom as f64).max(0.25),
                            layer_protection,
                        )
                    })
                    .or_else(|| find_terrain_at(layout, wp, layer_protection));
                let shift = ui.input(|i| i.modifiers.shift);
                let alt = ui.input(|i| i.modifiers.alt);
                if !shift {
                    selected.clear();
                }
                if let Some(id) = hit {
                    let group_ids: Vec<String> = if !alt {
                        layout
                            .pieces
                            .iter()
                            .find(|p| p.id == id)
                            .and_then(|p| p.group_id.clone())
                            .map(|gid| {
                                layout
                                    .pieces
                                    .iter()
                                    .filter(|p| {
                                        p.group_id.as_deref() == Some(gid.as_str())
                                            && is_layer_editable_with_protection(
                                                layout,
                                                p.layer_id.as_deref(),
                                                layer_protection,
                                            )
                                    })
                                    .map(|p| p.id.clone())
                                    .collect()
                            })
                            .unwrap_or_else(|| vec![id.clone()])
                    } else {
                        vec![id.clone()]
                    };
                    if shift && selected.contains(&id) {
                        for gid in &group_ids {
                            selected.remove(gid);
                        }
                    } else {
                        for gid in group_ids {
                            selected.insert(gid);
                        }
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

    if !sketcher.active && !ramp_pick_active {
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
                if terrain_brush.is_heightfield_mode() {
                    let shift_s = ui.input(|i| i.modifiers.shift);
                    let alt_i = ui.input(|i| i.modifiers.alt);
                    terrain_brush.is_drawing = true;
                    terrain_brush.flatten_target_mm = None;
                    terrain_brush.last_sculpt_world = None;
                    if terrain_brush.apply_live_sculpt_step(wp, layout, shift_s, alt_i) {
                        interaction.changed = true;
                    }
                } else if terrain_brush.start_stroke(wp, layout) {
                    interaction.changed = true;
                }
            } else if !on_plus_btn {
                if let Some((ref fid, handles)) = selected_flex_handles {
                    // Pick the closest handle within 15 px (preferring endpoint handles 3/4 on ties)
                    let mut best_hi: Option<(usize, f32)> = None;
                    for (hi, &hpos) in handles.iter().enumerate() {
                        let d = sp.distance(hpos);
                        if d <= 15.0 {
                            let score = if hi >= 3 { d - 2.0 } else { d };
                            if best_hi.map_or(true, |(_, bs)| score < bs) {
                                best_hi = Some((hi, score));
                            }
                        }
                    }
                    if let Some((hi, _)) = best_hi {
                        *dragging_flex_handle = Some((fid.clone(), hi));
                        interaction.push_undo = true;
                    }
                }
                if dragging_flex_handle.is_none() {
                    if let Some((ref rot_mid, rot_hpos)) = selected_model_rot_handle {
                        if sp.distance(rot_hpos) <= 14.0 {
                            *dragging_model_rot = Some(rot_mid.clone());
                            interaction.push_undo = true;
                        }
                    } else if let Some((pivot_w, rot_hpos, base_ang, ref single_pid, init_rot)) =
                        selected_track_rot_handle
                    {
                        if sp.distance(rot_hpos) <= 14.5 {
                            let start_ang =
                                (wp.y - pivot_w.y).atan2(wp.x - pivot_w.x).to_degrees();
                            *dragging_track_rot = Some(TrackRotationDrag {
                                pivot: pivot_w,
                                start_pointer_angle_deg: start_ang,
                                applied_delta_deg: 0.0,
                                base_handle_angle_deg: base_ang,
                                single_piece_id: single_pid.clone(),
                                initial_piece_rot_deg: init_rot,
                            });
                            interaction.push_undo = true;
                        }
                    }
                }
                if dragging_flex_handle.is_none()
                    && dragging_model_rot.is_none()
                    && dragging_track_rot.is_none()
                {
                    if measure_mode {
                        let alt_held = ui.input(|i| i.modifiers.alt);
                        let (pt, _) = snap_measure_point(wp, layout, alt_held, camera.zoom);
                        *measure_points = Some((pt, pt));
                        *measure_locked = false;
                    } else if let Some(hit) = find_building_at(layout, wp, layer_protection)
                        .or_else(|| {
                            find_piece_at(
                                layout,
                                cache,
                                wp,
                                22.0 / (camera.zoom as f64).max(0.25),
                                layer_protection,
                            )
                        })
                        .or_else(|| find_terrain_at(layout, wp, layer_protection))
                    {
                        if !selected.contains(&hit) {
                            let alt = ui.input(|i| i.modifiers.alt);
                            selected.clear();
                            let group_ids: Vec<String> = if !alt {
                                layout
                                    .pieces
                                    .iter()
                                    .find(|p| p.id == hit)
                                    .and_then(|p| p.group_id.clone())
                                    .map(|gid| {
                                        layout
                                            .pieces
                                            .iter()
                                            .filter(|p| {
                                                p.group_id.as_deref() == Some(gid.as_str())
                                                    && is_layer_editable_with_protection(
                                                        layout,
                                                        p.layer_id.as_deref(),
                                                        layer_protection,
                                                    )
                                            })
                                            .map(|p| p.id.clone())
                                            .collect()
                                    })
                                    .unwrap_or_else(|| vec![hit.clone()])
                            } else {
                                vec![hit]
                            };
                            for gid in group_ids {
                                selected.insert(gid);
                            }
                        }
                        if layer_protection {
                            let active_lid = active_layer_id;
                            selected.retain(|sid| {
                                layout
                                    .pieces
                                    .iter()
                                    .find(|p| &p.id == sid)
                                    .map(|p| p.layer_id.as_deref().unwrap_or("layer-1") == active_lid)
                                    .or_else(|| {
                                        layout
                                            .models
                                            .as_ref()
                                            .and_then(|ms| ms.iter().find(|m| &m.id == sid))
                                            .map(|m| m.layer_id.as_deref().unwrap_or("layer-1") == active_lid)
                                    })
                                    .unwrap_or(false)
                            });
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
                if terrain_brush.is_heightfield_mode() {
                    let shift_s = ui.input(|i| i.modifiers.shift);
                    let alt_i = ui.input(|i| i.modifiers.alt);
                    if terrain_brush.apply_live_sculpt_step(wp, layout, shift_s, alt_i) {
                        interaction.changed = true;
                    }
                } else if terrain_brush.move_stroke(wp, layout) {
                    interaction.changed = true;
                }
            }
        }
    } else if let Some((ref fid, handle_idx)) = dragging_flex_handle.clone() {
        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(sp) = response.interact_pointer_pos() {
                let wp = camera.screen_to_world(rect, sp);
                if handle_idx < 3 {
                    // Drag one of the 3 interior bend points (25%, 50%, 75%)
                    if let Some(p) = layout.pieces.iter_mut().find(|pc| &pc.id == fid) {
                        crate::catalog::apply_flex_spline_offset(p, catalogs, handle_idx, wp);
                        interaction.changed = true;
                    }
                } else {
                    // Drag either endpoint handle (handle 3 = c[1], handle 4 = c[0]) with magnetic C1 snapping + auto-shorten/bend
                    let moving_idx = if handle_idx == 3 { 1 } else { 0 };
                    let alt_bypass = ui.input(|i| i.modifiers.alt);
                    let snap_radius = (30.0 / (camera.zoom as f64).max(0.25)).clamp(22.0, 55.0);
                    let snap_target = if snapping_enabled && !alt_bypass {
                        find_best_flex_endpoint_snap_with_height(layout, catalogs, fid, moving_idx, wp, snap_radius)
                    } else {
                        None
                    };

                    let flex_layer_h = layout
                        .pieces
                        .iter()
                        .find(|pc| &pc.id == fid)
                        .and_then(|pc| {
                            let lid = pc.layer_id.as_deref().unwrap_or("layer-1");
                            layout
                                .layers
                                .as_ref()
                                .and_then(|ls| ls.iter().find(|l| l.id == lid))
                                .and_then(|l| l.height_mm)
                        })
                        .unwrap_or(0.0);

                    if let Some(p) = layout.pieces.iter_mut().find(|pc| &pc.id == fid) {
                        let snapped = if let Some((tpos, tang, target_wh)) = snap_target {
                            if crate::catalog::snap_flex_endpoint(p, catalogs, moving_idx, tpos, tang) {
                                let rel_target = ((target_wh - flex_layer_h) * 10.0).round() / 10.0;
                                let mut el = p.elevation.unwrap_or_default();
                                if moving_idx == 0 {
                                    el.start = rel_target;
                                } else {
                                    el.end = rel_target;
                                }
                                if el.start.abs() > 0.05 || el.end.abs() > 0.05 || p.elevation.is_some() {
                                    p.elevation = Some(el);
                                }
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        if !snapped {
                            crate::catalog::apply_flex_endpoint_drag(p, catalogs, moving_idx, wp);
                        }
                        interaction.changed = true;

                        // Live snap ring & length/angle readout badge
                        if let Some(c) = p.connectors.get(moving_idx) {
                            let ep_scr = camera.world_to_screen(rect, c.x, c.y);
                            let len_mm = p.custom_length.unwrap_or(0.0).round() as i32;
                            let ang_deg = p.custom_angle.unwrap_or(0.0).round() as i32;
                            let (badge_bg, badge_border, badge_txt) = if snapped {
                                painter.circle_stroke(
                                    ep_scr,
                                    13.0,
                                    Stroke::new(2.4_f32, Color32::from_rgb(16, 185, 129)),
                                );
                                (
                                    Color32::from_rgba_unmultiplied(6, 78, 59, 235),
                                    Color32::from_rgb(52, 211, 153),
                                    format!("SNAP · {len_mm} mm"),
                                )
                            } else {
                                (
                                    Color32::from_rgba_unmultiplied(15, 23, 42, 225),
                                    Color32::from_rgb(245, 158, 11),
                                    format!("{len_mm} mm · {ang_deg}°"),
                                )
                            };
                            let pill = Rect::from_min_size(
                                ep_scr + egui::vec2(14.0, -26.0),
                                egui::vec2(118.0, 22.0),
                            );
                            painter.rect_filled(pill, 6.0, badge_bg);
                            painter.rect_stroke(
                                pill,
                                6.0,
                                Stroke::new(1.2_f32, badge_border),
                                egui::StrokeKind::Inside,
                            );
                            painter.text(
                                pill.center(),
                                egui::Align2::CENTER_CENTER,
                                badge_txt,
                                egui::FontId::proportional(11.5),
                                Color32::WHITE,
                            );
                        }
                    }
                }
            }
        }
    } else if measure_mode && response.dragged_by(egui::PointerButton::Primary) {
        if let (Some(sp), Some((a, _))) = (response.interact_pointer_pos(), *measure_points) {
            let alt_held = ui.input(|i| i.modifiers.alt);
            let raw = camera.screen_to_world(rect, sp);
            let (pt, _) = snap_measure_point(raw, layout, alt_held, camera.zoom);
            *measure_points = Some((a, pt));
            *measure_locked = false;
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
    } else if let Some(ref mut drag) = *dragging_track_rot {
        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(sp) = response.interact_pointer_pos() {
                let wp = camera.screen_to_world(rect, sp);
                let cur_ang = (wp.y - drag.pivot.y).atan2(wp.x - drag.pivot.x).to_degrees();
                let raw_delta = unwrap_drag_angle_delta(
                    cur_ang,
                    drag.start_pointer_angle_deg,
                    drag.applied_delta_deg,
                );
                let free_rot = ui.input(|i| i.modifiers.shift);
                let desired_delta = snap_drag_rotation_delta(raw_delta, rotation_step, free_rot);
                let step_delta = desired_delta - drag.applied_delta_deg;
                if step_delta.abs() > 1e-6
                    && rotate_selection_around_pivot(
                        layout,
                        selected,
                        layer_protection,
                        drag.pivot,
                        step_delta,
                    )
                {
                    drag.applied_delta_deg = desired_delta;
                    interaction.changed = true;
                }
                let readout = if drag.single_piece_id.is_some() {
                    let abs_deg =
                        norm_angle_deg(drag.initial_piece_rot_deg + drag.applied_delta_deg);
                    format_rotation_readout(abs_deg, false, free_rot)
                } else {
                    format_rotation_readout(drag.applied_delta_deg, true, free_rot)
                };
                let pill = Rect::from_min_size(
                    sp + egui::vec2(16.0, -26.0),
                    egui::vec2(68.0, 22.0),
                );
                painter.rect_filled(
                    pill,
                    6.0,
                    Color32::from_rgba_unmultiplied(15, 23, 42, 225),
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
                    egui::FontId::proportional(11.5),
                    Color32::WHITE,
                );
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
                let active_lid = active_layer_id;
                for p in &mut layout.pieces {
                    if selected.contains(&p.id)
                        && (!layer_protection
                            || p.layer_id.as_deref().unwrap_or("layer-1") == active_lid)
                    {
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
                        if selected.contains(&m.id)
                            && (!layer_protection
                                || m.layer_id.as_deref().unwrap_or("layer-1") == active_lid)
                        {
                            m.transform.x += dx;
                            m.transform.y += dy;
                        }
                    }
                }
                *drag_last_world = Some(cur_w);
                interaction.changed = true;
            }
        }
        // Live magnetic snap preview ring while dragging (green/cyan if same level, yellow if different level)
        if snapping_enabled {
            let snap_radius = (42.0 / (camera.zoom as f64).max(0.2)).clamp(35.0, 95.0);
            let same_preview = find_best_snap_candidate(layout, catalogs, selected, snap_radius)
                .map(|cand| (Vec2 { x: cand.src_x, y: cand.src_y }, Vec2 { x: cand.dst_x, y: cand.dst_y }))
                .or_else(|| {
                    crate::building_snap::find_best_track_to_building_snap(layout, catalogs, selected, snap_radius)
                        .map(|bc| (Vec2 { x: bc.src_x, y: bc.src_y }, Vec2 { x: bc.dst_x, y: bc.dst_y }))
                })
                .or_else(|| {
                    crate::building_snap::preview_selected_model_snap(layout, catalogs, selected)
                        .and_then(|(s, d, same)| if same { Some((s, d)) } else { None })
                });

            if let Some((src, dst)) = same_preview {
                let s1 = camera.world_to_screen(rect, src.x, src.y);
                let s2 = camera.world_to_screen(rect, dst.x, dst.y);
                let cyan = Color32::from_rgb(56, 189, 248);
                painter.line_segment([s1, s2], Stroke::new(1.8_f32, cyan));
                painter.circle_stroke(s2, 10.0, Stroke::new(2.2_f32, cyan));
                painter.circle_filled(s2, 4.0, Color32::from_rgb(34, 197, 94));
            } else {
                let diff_preview = find_best_snap_candidate_any_level(layout, catalogs, selected, snap_radius)
                    .and_then(|cand| {
                        if !cand.same_level {
                            Some((Vec2 { x: cand.src_x, y: cand.src_y }, Vec2 { x: cand.dst_x, y: cand.dst_y }))
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        crate::building_snap::find_best_track_to_building_snap_any_level(
                            layout,
                            catalogs,
                            selected,
                            snap_radius,
                        )
                        .and_then(|bc| {
                            if !bc.same_level {
                                Some((Vec2 { x: bc.src_x, y: bc.src_y }, Vec2 { x: bc.dst_x, y: bc.dst_y }))
                            } else {
                                None
                            }
                        })
                    })
                    .or_else(|| {
                        crate::building_snap::preview_selected_model_snap(layout, catalogs, selected)
                            .and_then(|(s, d, same)| if !same { Some((s, d)) } else { None })
                    });

                if let Some((src, dst)) = diff_preview {
                    let s1 = camera.world_to_screen(rect, src.x, src.y);
                    let s2 = camera.world_to_screen(rect, dst.x, dst.y);
                    let yellow = Color32::from_rgb(250, 204, 21);
                    painter.line_segment([s1, s2], Stroke::new(1.8_f32, yellow));
                    painter.circle_stroke(s2, 10.5, Stroke::new(2.4_f32, yellow));
                    painter.circle_filled(s2, 5.0, Color32::from_rgb(234, 179, 8));
                }
            }
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        if measure_mode {
            if let Some((a, b)) = *measure_points {
                if (b.x - a.x).hypot(b.y - a.y) > 4.0 {
                    *measure_locked = true;
                }
            }
        }
        if terrain_brush.is_drawing {
            let was_erase_or_hf = terrain_brush.mode == TerrainBrushMode::Erase
                || terrain_brush.is_heightfield_mode();
            let active_lid = active_layer_id.to_string();
            if let Some(created_id) =
                terrain_brush.finish_stroke(layout, next_piece_seq, Some(active_lid.as_str()), is_de)
            {
                selected.clear();
                selected.insert(created_id);
                interaction.changed = true;
            } else if was_erase_or_hf {
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
                if !is_layer_editable_with_protection(layout, p.layer_id.as_deref(), layer_protection) {
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
                    if !is_layer_editable_with_protection(layout, m.layer_id.as_deref(), layer_protection) {
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
        if dragging_track_rot.take().is_some() {
            railbench_core::snapping::update_layout_connections(layout, 4.0, 35.0, 1.0);
            crate::catalog::disconnect_cross_type_connections(layout, catalogs);
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

}
