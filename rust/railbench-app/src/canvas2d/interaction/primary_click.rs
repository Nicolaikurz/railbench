use super::super::*;
use super::snap_measure_point;

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_ramp_and_primary_click(
    ui: &mut egui::Ui,
    rect: Rect,
    response: &egui::Response,
    painter: &egui::Painter,
    layout: &mut Layout,
    cache: &Canvas2DCache,
    catalogs: &[Catalog],
    camera: &Camera2D,
    selected: &mut HashSet<String>,
    measure_points: &mut Option<(Vec2, Vec2)>,
    measure_locked: &mut bool,
    layer_protection: bool,
    board_edit_enabled: bool,
    selection_scope: crate::SelectionScope,
    measure_mode: bool,
    terrain_brush: &mut TerrainBrushState,
    is_de: bool,
    active_board_id: &mut Option<String>,
    sketcher: &BoardSketcherState,
    ramp_overlay: &mut Option<HeightPanelOverlay<'_>>,
    ramp_pick_active: bool,
    board_header_rects: &[(String, Rect)],
    plus_buttons: &[(String, usize, Pos2, bool)],
    accessory_buttons: &[TrackAccessoryButton],
    selected_model_rot_handle: Option<&(String, Pos2)>,
    selected_track_rot_handle: Option<&(Vec2, Pos2, f64, Option<String>, f64)>,
    interaction: &mut CanvasInteraction,
) {
    if let Some(ref ov) = *ramp_overlay {
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
            let pill_w = (label.len() as f32 * 6.4 + 16.0).max(74.0);
            let pill = Rect::from_min_size(sp + egui::vec2(14.0, -11.0), egui::vec2(pill_w, 22.0));
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

        if ov.tab == 1 {
            let chain = crate::ramp_tools::resolve_ramp_chain(
                layout,
                selected,
                ov.ramp_sel.start_piece.as_deref(),
                ov.ramp_sel.end_piece.as_deref(),
            );
            let segs = crate::ramp_tools::ramp_chain_direction_segments(layout, &chain);

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

            let start_pos_w = segs.first().map(|(a, _)| *a).or_else(|| {
                ov.ramp_sel.start_piece.as_deref().and_then(|sid| {
                    layout.pieces.iter().find(|p| p.id == sid).map(|p| {
                        p.connectors
                            .first()
                            .map(|c| Vec2 { x: c.x, y: c.y })
                            .unwrap_or(Vec2 { x: p.transform.x, y: p.transform.y })
                    })
                })
            });
            let end_pos_w = segs.last().map(|(_, b)| *b).or_else(|| {
                ov.ramp_sel.end_piece.as_deref().and_then(|eid| {
                    layout.pieces.iter().find(|p| p.id == eid).map(|p| {
                        p.connectors
                            .last()
                            .map(|c| Vec2 { x: c.x, y: c.y })
                            .unwrap_or(Vec2 { x: p.transform.x, y: p.transform.y })
                    })
                })
            });

            if let Some(a_w) = start_pos_w {
                draw_ramp_pin(a_w, "A", format!("Start {:.0} mm", *ov.ramp_start_h), Color32::from_rgb(16, 185, 129));
            }
            if let Some(b_w) = end_pos_w {
                draw_ramp_pin(b_w, "B", format!("End {:.0} mm", *ov.ramp_end_h), Color32::from_rgb(245, 158, 11));
            }

            if let Some(picking_start) = ov.ramp_sel.pick_mode {
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
                    egui::vec2(420.0, 28.0),
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
        } else if ov.tab == 2 {
            // Live ghost preview of automatically generated ramp / curve / helix
            if !ov.gen_preview_pieces.is_empty() {
                let mut prev_anchor = ov
                    .gen_anchor_piece_id
                    .as_deref()
                    .and_then(|aid| layout.pieces.iter().find(|p| p.id == aid))
                    .and_then(|ap| {
                        let ci = ov.gen_conn_idx.unwrap_or(0).min(ap.connectors.len().saturating_sub(1));
                        ap.connectors.get(ci).map(|c| Vec2 { x: c.x, y: c.y })
                    });
                let mut first_in: Option<Vec2> = prev_anchor;
                let mut last_out: Option<Vec2> = None;
                let ghost_roadbed_w = (22.0 * camera.zoom).clamp(4.0, 26.0);
                let ghost_fill = Color32::from_rgba_unmultiplied(16, 185, 129, 68);
                let ghost_line = Color32::from_rgba_unmultiplied(56, 189, 248, 225);
                let ghost_arrow = Color32::from_rgb(52, 211, 153);

                for gp in ov.gen_preview_pieces {
                    if gp.connectors.len() >= 2 {
                        let c0 = Vec2 { x: gp.connectors[0].x, y: gp.connectors[0].y };
                        let c1 = Vec2 { x: gp.connectors[1].x, y: gp.connectors[1].y };
                        let (p_in, p_out) = if let Some(prev) = prev_anchor {
                            if (c0.x - prev.x).hypot(c0.y - prev.y) <= (c1.x - prev.x).hypot(c1.y - prev.y) {
                                (c0, c1)
                            } else {
                                (c1, c0)
                            }
                        } else {
                            (c0, c1)
                        };
                        if first_in.is_none() {
                            first_in = Some(p_in);
                        }
                        prev_anchor = Some(p_out);
                        last_out = Some(p_out);

                        let entry = find_entry(catalogs, &gp.catalog_id);
                        if entry.is_some_and(|e| e.kind == "curve") {
                            let c0 = &gp.connectors[0];
                            let c1 = &gp.connectors[1];
                            let arc_pts = sample_arc_between_connectors(
                                c0.x, c0.y, c0.angle, c1.x, c1.y, c1.angle, 14,
                            );
                            let s_pts: Vec<Pos2> = arc_pts
                                .into_iter()
                                .map(|pt| camera.world_to_screen(rect, pt.x, pt.y))
                                .collect();
                            painter.add(egui::Shape::line(
                                s_pts.clone(),
                                Stroke::new(ghost_roadbed_w, ghost_fill),
                            ));
                            painter.add(egui::Shape::line(
                                s_pts.clone(),
                                Stroke::new(2.8_f32, ghost_line),
                            ));
                            if s_pts.len() >= 3 {
                                let mid_idx = s_pts.len() / 2;
                                let (a, b) = if (s_pts[0] - camera.world_to_screen(rect, p_in.x, p_in.y)).length_sq()
                                    <= (s_pts[s_pts.len() - 1] - camera.world_to_screen(rect, p_in.x, p_in.y)).length_sq()
                                {
                                    (s_pts[mid_idx - 1], s_pts[mid_idx])
                                } else {
                                    (s_pts[mid_idx], s_pts[mid_idx - 1])
                                };
                                let dir = b - a;
                                if dir.length() > 1.0 {
                                    let u = dir.normalized();
                                    let perp = egui::vec2(-u.y, u.x);
                                    let mid = s_pts[mid_idx];
                                    painter.add(egui::Shape::convex_polygon(
                                        vec![mid + u * 6.5, mid - u * 4.0 + perp * 4.5, mid - u * 4.0 - perp * 4.5],
                                        ghost_arrow,
                                        Stroke::new(0.9, Color32::from_rgb(15, 23, 42)),
                                    ));
                                }
                            }
                        } else {
                            let s_in = camera.world_to_screen(rect, p_in.x, p_in.y);
                            let s_out = camera.world_to_screen(rect, p_out.x, p_out.y);
                            painter.line_segment([s_in, s_out], Stroke::new(ghost_roadbed_w, ghost_fill));
                            painter.line_segment([s_in, s_out], Stroke::new(2.8_f32, ghost_line));
                            let dir = s_out - s_in;
                            if dir.length() > 14.0 {
                                let u = dir.normalized();
                                let perp = egui::vec2(-u.y, u.x);
                                let mid = s_in + dir * 0.55;
                                painter.add(egui::Shape::convex_polygon(
                                    vec![mid + u * 6.5, mid - u * 4.0 + perp * 4.5, mid - u * 4.0 - perp * 4.5],
                                    ghost_arrow,
                                    Stroke::new(0.9, Color32::from_rgb(15, 23, 42)),
                                ));
                            }
                        }
                    }
                }

                if let Some(a_w) = first_in {
                    draw_ramp_pin(a_w, "A", format!("{:.0} mm", ov.gen_start_h), Color32::from_rgb(16, 185, 129));
                }
                if let Some(b_w) = last_out {
                    draw_ramp_pin(b_w, "B", format!("{:.0} mm", ov.gen_target_h), Color32::from_rgb(245, 158, 11));
                }
            }

            if *ov.gen_pick_mode {
                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                }
                let banner = Rect::from_center_size(
                    Pos2::new(rect.center().x, rect.min.y + 40.0),
                    egui::vec2(440.0, 28.0),
                );
                let msg = if is_de {
                    "Start-Gleis wählen: Klicke ein Gleis im Plan (Esc = Abbrechen)"
                } else {
                    "Pick Start Track: Click a track piece on the canvas (Esc to cancel)"
                };
                painter.rect_filled(banner, 8.0, Color32::from_rgba_unmultiplied(15, 23, 42, 235));
                painter.rect_stroke(
                    banner,
                    8.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(56, 189, 248)),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    banner.center(),
                    egui::Align2::CENTER_CENTER,
                    msg,
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );
            }
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
                if let Some(ref mut ov) = *ramp_overlay {
                    if ov.tab == 1 {
                        match ov.ramp_sel.pick_mode {
                            Some(true) => {
                                ov.ramp_sel.start_piece = Some(pid.clone());
                                if let Some(h) = crate::ramp_tools::piece_world_height_mm(layout, &pid, false) {
                                    *ov.ramp_start_h = h;
                                }
                                ov.ramp_sel.pick_mode = if ov.ramp_sel.end_piece.is_none() {
                                    Some(false)
                                } else {
                                    None
                                };
                            }
                            Some(false) => {
                                ov.ramp_sel.end_piece = Some(pid.clone());
                                if let Some(h) = crate::ramp_tools::piece_world_height_mm(layout, &pid, true) {
                                    *ov.ramp_end_h = h;
                                }
                                ov.ramp_sel.pick_mode = None;
                            }
                            None => {}
                        }
                    } else if ov.tab == 2 && *ov.gen_pick_mode {
                        if let Some((chosen_pid, chosen_ci)) =
                            crate::ramp_tools::find_nearest_open_connector_in_chain(layout, &pid, wp)
                        {
                            *ov.gen_anchor_piece_id = Some(chosen_pid);
                            *ov.gen_conn_idx = Some(chosen_ci);
                            *ov.gen_pick_mode = false;
                        }
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
            let on_rot_knob = (selection_scope.allows_any_piece() || selection_scope.allows_buildings())
                && (selected_track_rot_handle
                    .as_ref()
                    .is_some_and(|(_, hs, _, _, _)| sp.distance(*hs) <= 14.0)
                    || selected_model_rot_handle
                        .as_ref()
                        .is_some_and(|(_, hs)| sp.distance(*hs) <= 14.0));
            if on_rot_knob {
                // Keep selection intact when clicking directly on the rotation handle
            } else if let Some(btn) = selection_scope
                .allows_rail_accessories()
                .then(|| hit_test_accessory_button(accessory_buttons, sp))
                .flatten()
            {
                match btn.kind {
                    TrackAccessoryButtonKind::Uncoupler => {
                        if ui.input(|i| i.modifiers.shift) {
                            interaction.toggle_uncoupler_armed = Some(btn.piece_id.clone());
                        } else {
                            interaction.trigger_uncoupler_piece = Some(btn.piece_id.clone());
                        }
                    }
                    TrackAccessoryButtonKind::Feeder | TrackAccessoryButtonKind::Isolator => {
                        interaction.push_undo = true;
                        if toggle_track_accessory_on_piece(layout, &btn.piece_id, btn.kind) {
                            interaction.changed = true;
                        }
                    }
                }
            } else if let Some((pid, cidx, _, _)) = selection_scope
                .allows_any_piece()
                .then(|| {
                    plus_buttons
                        .iter()
                        .find(|(_, _, ppos, _)| sp.distance(*ppos) <= 13.5)
                })
                .flatten()
            {
                interaction.quick_add_at = Some((pid.clone(), *cidx));
            } else {
                let wp = camera.screen_to_world(rect, sp);
                let hit = find_canvas_object_at(
                    layout,
                    cache,
                    catalogs,
                    wp,
                    22.0 / (camera.zoom as f64).max(0.25),
                    layer_protection,
                    selection_scope,
                );
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
                } else if board_edit_enabled && selection_scope == crate::SelectionScope::Boards {
                    if let Some((bid, _)) = find_board_drag_hit(
                        board_edit_enabled,
                        layout,
                        wp,
                        sp,
                        camera.zoom,
                        board_header_rects,
                        true,
                    ) {
                        *active_board_id = Some(bid);
                    }
                }
            }
        }
    }
}
