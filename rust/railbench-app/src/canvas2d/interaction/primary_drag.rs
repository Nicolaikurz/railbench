use super::super::*;
use super::snap_measure_point;

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_primary_drag(
    ui: &mut egui::Ui,
    rect: Rect,
    response: &egui::Response,
    painter: &egui::Painter,
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
    measure_mode: bool,
    terrain_brush: &mut TerrainBrushState,
    is_de: bool,
    next_piece_seq: &mut u64,
    active_board_id: &mut Option<String>,
    dragging_board: &mut Option<(String, Vec2, f64, f64)>,
    board_snap_guides: &mut Vec<BaseboardSnapGuide>,
    dragging_model_rot: &mut Option<String>,
    dragging_track_rot: &mut Option<TrackRotationDrag>,
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
    if response.drag_started_by(egui::PointerButton::Primary) {
        *dragging_active = false;
        *drag_last_world = None;
        *marquee_start = None;
        *dragging_flex_handle = None;
        *dragging_model_rot = None;
        *dragging_track_rot = None;
        *dragging_board = None;
        board_snap_guides.clear();

        let start_sp = ui
            .input(|i| i.pointer.press_origin())
            .filter(|p| rect.contains(*p))
            .or_else(|| response.interact_pointer_pos());
        if let Some(sp) = start_sp {
            let wp = camera.screen_to_world(rect, sp);
            let on_plus_btn = !terrain_brush.active
                && selection_scope.allows_any_piece()
                && !measure_mode
                && plus_buttons
                    .iter()
                    .any(|(_, _, ppos, _)| sp.distance(*ppos) <= 13.5);
            let on_accessory_btn = !terrain_brush.active
                && selection_scope.allows_rail_accessories()
                && !measure_mode
                && hit_test_accessory_button(accessory_buttons, sp).is_some();
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
            } else if !on_plus_btn && !on_accessory_btn {
                if selection_scope.allows_any_piece() || selection_scope.allows_buildings() {
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
                    } else if let Some(hit) = find_canvas_object_at(
                        layout,
                        cache,
                        catalogs,
                        wp,
                        22.0 / (camera.zoom as f64).max(0.25),
                        layer_protection,
                        selection_scope,
                    ) {
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
                                                    && selection_scope.allows_piece(catalogs, p)
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
                        board_edit_enabled,
                        layout,
                        wp,
                        sp,
                        camera.zoom,
                        board_header_rects,
                        true,
                    ) {
                        *active_board_id = Some(bid.clone());
                        *dragging_board = Some((bid, wp, bounds.x, bounds.y));
                        interaction.push_undo = true;
                    } else if selection_scope != crate::SelectionScope::Boards {
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
                if selection_scope.allows_any_piece() {
                    for p in &mut layout.pieces {
                        if selected.contains(&p.id)
                            && selection_scope.allows_piece(catalogs, p)
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
                }
                if let Some(models) = &mut layout.models {
                    for m in models {
                        if selected.contains(&m.id)
                            && selection_scope.allows_model(m)
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

    let stopped_primary = response.drag_stopped_by(egui::PointerButton::Primary);
    let lost_primary = !ui.input(|i| i.pointer.primary_down());
    if stopped_primary
        || (lost_primary
            && (*dragging_active
                || marquee_start.is_some()
                || dragging_flex_handle.is_some()
                || dragging_model_rot.is_some()
                || dragging_track_rot.is_some()
                || dragging_board.is_some()))
    {
        if stopped_primary && measure_mode {
            if let Some((a, b)) = *measure_points {
                if (b.x - a.x).hypot(b.y - a.y) > 4.0 {
                    *measure_locked = true;
                }
            }
        }
        if stopped_primary && terrain_brush.is_drawing {
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
        if let Some(start) = marquee_start.take() {
            if stopped_primary && !*dragging_active {
                if let Some(end) = response
                    .interact_pointer_pos()
                    .or_else(|| ui.input(|i| i.pointer.latest_pos()))
                {
                    let shift = ui.input(|i| i.modifiers.shift);
                    if (end - start).length() >= 4.0 {
                        let mrect = Rect::from_two_pos(start, end);
                        if !shift {
                            selected.clear();
                        }
                        if selection_scope.allows_any_piece() {
                            for p in &layout.pieces {
                                if !selection_scope.allows_piece(catalogs, p)
                                    || !is_layer_editable_with_protection(
                                        layout,
                                        p.layer_id.as_deref(),
                                        layer_protection,
                                    )
                                {
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
                        }
                        if let Some(models) = &layout.models {
                            for m in models {
                                if !selection_scope.allows_model(m)
                                    || !is_layer_editable_with_protection(
                                        layout,
                                        m.layer_id.as_deref(),
                                        layer_protection,
                                    )
                                {
                                    continue;
                                }
                                let sp = camera.world_to_screen(rect, m.transform.x, m.transform.y);
                                if mrect.contains(sp) {
                                    selected.insert(m.id.clone());
                                }
                            }
                        }
                    } else if !shift {
                        selected.clear();
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
}
