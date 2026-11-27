use super::*;

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
    measure_locked: &mut bool,
    dragging_catalog: Option<&CatalogEntry>,
    dragging_model_template: Option<&Model3D>,
    collab_users: &[crate::collab::CollabUser],
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
    board_edit_enabled: bool,
    selection_scope: crate::SelectionScope,
    sketcher: &mut BoardSketcherState,
    dragging_model_rot: &mut Option<String>,
    dragging_track_rot: &mut Option<TrackRotationDrag>,
    ramp_overlay: Option<HeightPanelOverlay<'_>>,
    vehicle_sim: &crate::vehicle_sim::VehicleSimState,
    cad_tools: &mut crate::cad_tools::CadToolsState,
) -> CanvasInteraction {
    let mut interaction = CanvasInteraction::default();

    let avail = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    // Background (matches variables.css --canvas-bg)
    let bg = crate::theme::palette(dark_mode).canvas_bg;
    painter.rect_filled(rect, 0.0, bg);

    // Scroll wheel: Shift+Wheel adjusts terrain brush radius, Ctrl+Wheel adjusts strength when active; otherwise zoom
    if response.hovered() {
        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_y.abs() > 0.01 {
            if terrain_brush.active && ui.input(|i| i.modifiers.shift) {
                terrain_brush.adjust_radius(if scroll_y > 0.0 { 20.0 } else { -20.0 });
            } else if terrain_brush.active && ui.input(|i| i.modifiers.ctrl || i.modifiers.mac_cmd) {
                terrain_brush.adjust_strength(if scroll_y > 0.0 { 0.08 } else { -0.08 });
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
        board_edit_enabled,
    );

    // 2B. Arm-Reachability Heatmap Overlay (under tracks so track geometry stays sharp)
    if cad_tools.reachability_enabled {
        cad_tools.last_unreachable_pct = crate::cad_tools::draw_reachability_heatmap_2d(
            &painter,
            rect,
            camera,
            layout,
            cad_tools.max_reach_mm,
            is_de,
        );
    }

    // 2C. Legacy layout-level continuous heightfield fallback (only when layout.layers is empty/None)
    if let Some(ref hf_map) = cache.heightfield_map {
        super::terrain_maps::draw_heightfield_map_2d(&painter, rect, camera, hf_map, false);
    }

    let active_layer_id = layout
        .active_layer_id
        .as_deref()
        .unwrap_or("layer-1")
        .to_string();

    let mut accessory_buttons: Vec<TrackAccessoryButton> = Vec::new();

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

        // 2.5A. Layer heightfield & ground terrain render BELOW tracks so tracks under mountains/water stay visible
        if let Some(hf_map) = cache.layer_heightfield_maps.get(lid.as_str()) {
            super::terrain_maps::draw_heightfield_map_2d(&painter, rect, camera, hf_map, !is_active_layer);
        }
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
                    cache.model_mtn_maps.get(mi).and_then(|o| o.as_deref()),
                    cache.model_ground_maps.get(mi).and_then(|o| o.as_deref()),
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
                let is_editable = is_layer_editable_with_protection(
                    layout,
                    piece.layer_id.as_deref(),
                    layer_protection,
                );
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
                    is_editable,
                    color_mode,
                    lcolor.as_deref(),
                    show_labels,
                    power_sim_enabled,
                    piece_power,
                    vehicle_sim.turnout_states.get(&piece.id).copied().unwrap_or(0),
                    vehicle_sim.turntable_angles_deg.get(&piece.id).copied().unwrap_or(0.0),
                    vehicle_sim.transfer_offsets_mm.get(&piece.id).copied(),
                    Some((
                        vehicle_sim.is_uncoupler_raised(&piece.id),
                        vehicle_sim.armed_uncouplers.contains(&piece.id),
                    )),
                    &mut accessory_buttons,
                );
            }
        }

        // 3.5. Buildings, trees, signals & lamps render ABOVE tracks on this layer
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
                    cache.model_mtn_maps.get(mi).and_then(|o| o.as_deref()),
                    cache.model_ground_maps.get(mi).and_then(|o| o.as_deref()),
                    is_sel,
                    is_active_layer,
                    show_labels,
                );
            }
        }
    }

    // 3.8. Live vehicles (locomotives, coupled wagons, cars, trucks, buses)
    render_vehicles::draw_vehicles_2d(&painter, rect, camera, vehicle_sim, &layout.scale);

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

    // 3B. NEM 102/103 Clearance & Headroom Conflict Markers
    if cad_tools.nem_check_enabled {
        crate::cad_tools::draw_nem_clearance_overlay_2d(
            &painter,
            rect,
            camera,
            &cad_tools.nem_violations,
            is_de,
        );
    }

    // 4. Connectors (snap points) & Connector '+' Quick-Add buttons
    let mut plus_buttons: Vec<(String, usize, Pos2, bool)> = Vec::new();
    let hover_pos = ui.input(|i| i.pointer.hover_pos());

    let mut hovered_open_conn: Option<(String, usize)> = None;
    if show_connectors
        && !terrain_brush.active
        && selection_scope.allows_any_piece()
        && !measure_mode
        && dragging_catalog.is_none()
        && dragging_model_template.is_none()
    {
        if let Some(hp) = hover_pos {
            let hit_r = (16.0 * camera.zoom.sqrt()).clamp(15.0, 24.0);
            let hp_world = camera.screen_to_world(rect, hp);
            let pad_world = (hit_r as f64 / camera.zoom.max(0.04) as f64) + 32.0;
            let mut best_d = f32::INFINITY;
            for (pidx, piece) in layout.pieces.iter().enumerate() {
                if !selection_scope.allows_piece(catalogs, piece) {
                    continue;
                }
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
                let piece_in_scope = selection_scope.allows_piece(catalogs, piece);
                let is_hovered_conn = hovered_open_conn
                    .as_ref()
                    .map_or(false, |(pid, ci)| pid == &piece.id && *ci == cidx);
                let may_draw_plus = piece_in_scope
                    && editable
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
    let selected_flex_handles: Option<(String, [Pos2; 5])> =
        if selection_scope.allows_any_piece() && selected.len() == 1 {
            selected.iter().next().and_then(|sid| {
                let p = cache
                    .piece_id_to_idx
                    .get(sid.as_str())
                    .and_then(|&idx| layout.pieces.get(idx))
                    .or_else(|| layout.pieces.iter().find(|pc| &pc.id == sid))?;
                if !selection_scope.allows_piece(catalogs, p)
                    || !is_layer_editable_with_protection(layout, p.layer_id.as_deref(), layer_protection)
                {
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
        if !sketcher.active && !terrain_brush.active && selection_scope.allows_buildings() && !measure_mode && selected.len() == 1 {
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

    // 4.7. Selected Track (single or multi-piece) rotation handle point
    let selected_track_rot_handle: Option<(Vec2, Pos2, f64, Option<String>, f64)> =
        if !sketcher.active && !terrain_brush.active && selection_scope.allows_any_piece() && !measure_mode && !selected.is_empty() {
            compute_track_selection_rotation_handle(
                layout,
                selected,
                layer_protection,
                camera.zoom,
                dragging_track_rot.as_ref(),
            )
            .map(|(pivot_w, handle_w, base_ang, single_pid, init_rot)| {
                let ps = camera.world_to_screen(rect, pivot_w.x, pivot_w.y);
                let hs = camera.world_to_screen(rect, handle_w.x, handle_w.y);
                let hovered_or_drag = dragging_track_rot.is_some()
                    || hover_pos.is_some_and(|hp| hp.distance(hs) <= 14.0);
                if dragging_track_rot.is_some() {
                    let orbit_r = ps.distance(hs).max(16.0);
                    painter.circle_stroke(
                        ps,
                        orbit_r,
                        Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 75)),
                    );
                }
                painter.circle_filled(ps, 3.5, Color32::from_rgb(15, 23, 42));
                painter.circle_stroke(ps, 3.8, Stroke::new(1.5_f32, Color32::from_rgb(56, 189, 248)));
                painter.line_segment(
                    [ps, hs],
                    Stroke::new(1.6_f32, Color32::from_rgb(56, 189, 248)),
                );
                let knob_r = if hovered_or_drag { 8.5 } else { 7.5 };
                painter.circle_filled(hs, knob_r, Color32::from_rgb(245, 158, 11));
                painter.circle_stroke(hs, knob_r + 1.2, Stroke::new(1.8_f32, Color32::WHITE));
                if hovered_or_drag {
                    ui.ctx().set_cursor_icon(if dragging_track_rot.is_some() {
                        egui::CursorIcon::Grabbing
                    } else {
                        egui::CursorIcon::Grab
                    });
                }
                (pivot_w, hs, base_ang, single_pid, init_rot)
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
            TerrainBrushMode::SculptRaise => Color32::from_rgb(34, 197, 94),
            TerrainBrushMode::SculptLower => Color32::from_rgb(249, 115, 22),
            TerrainBrushMode::SculptSmooth => Color32::from_rgb(56, 189, 248),
            TerrainBrushMode::SculptFlatten => Color32::from_rgb(234, 179, 8),
            TerrainBrushMode::SculptGrade => Color32::from_rgb(168, 85, 247),
            TerrainBrushMode::SculptBiome => Color32::from_rgb(20, 184, 166),
            TerrainBrushMode::SculptReset | TerrainBrushMode::Erase => Color32::from_rgb(239, 68, 68),
            TerrainBrushMode::Paint => Color32::from_rgb(34, 197, 94),
            TerrainBrushMode::Extend => Color32::from_rgb(14, 165, 233),
            TerrainBrushMode::Stamp => Color32::from_rgb(245, 158, 11),
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

    if !sketcher.active
        && !measure_mode
        && !terrain_brush.active
        && selection_scope.allows_rail_accessories()
        && dragging_catalog.is_none()
        && dragging_model_template.is_none()
    {
        if let Some(hp) = hover_pos.filter(|sp| rect.contains(*sp)) {
            if let Some(btn) = hit_test_accessory_button(&accessory_buttons, hp) {
                painter.circle_stroke(
                    btn.screen_center,
                    btn.radius_px + 2.5,
                    Stroke::new(2.0_f32, Color32::from_rgb(56, 189, 248)),
                );
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                let tip_txt = match (btn.kind, btn.active, is_de) {
                    (TrackAccessoryButtonKind::Feeder, true, true) => {
                        "Einspeisung aktiv — Klicken zum Ausschalten"
                    }
                    (TrackAccessoryButtonKind::Feeder, true, false) => {
                        "Power feed active — Click to disable"
                    }
                    (TrackAccessoryButtonKind::Feeder, false, true) => {
                        "Einspeisung aus — Klicken zum Einschalten"
                    }
                    (TrackAccessoryButtonKind::Feeder, false, false) => {
                        "Power feed off — Click to enable"
                    }
                    (TrackAccessoryButtonKind::Isolator, true, true) => {
                        "Trennstrecke aktiv — Klicken zum Ausschalten"
                    }
                    (TrackAccessoryButtonKind::Isolator, true, false) => {
                        "Isolator active — Click to disable"
                    }
                    (TrackAccessoryButtonKind::Isolator, false, true) => {
                        "Trennstrecke aus — Klicken zum Einschalten"
                    }
                    (TrackAccessoryButtonKind::Isolator, false, false) => {
                        "Isolator off — Click to enable"
                    }
                    (TrackAccessoryButtonKind::Uncoupler, true, true) => {
                        "Entkupplungsgleis aktiv — Klick: Entkuppeln · Shift+Klick: Auto"
                    }
                    (TrackAccessoryButtonKind::Uncoupler, true, false) => {
                        "Uncoupler active — Click: Uncouple · Shift+Click: Auto"
                    }
                    (TrackAccessoryButtonKind::Uncoupler, false, true) => {
                        "Entkupplungsgleis — Klick: Entkuppeln · Shift+Klick: Auto-Modus"
                    }
                    (TrackAccessoryButtonKind::Uncoupler, false, false) => {
                        "Uncoupler track — Click: Uncouple · Shift+Click: Auto mode"
                    }
                };
                let galley = painter.layout_no_wrap(
                    tip_txt.to_string(),
                    FontId::proportional(11.0),
                    Color32::WHITE,
                );
                let pill_w = galley.size().x + 14.0;
                let pill_h = 22.0;
                let tip_cx = btn
                    .screen_center
                    .x
                    .clamp(rect.min.x + pill_w * 0.5 + 6.0, rect.max.x - pill_w * 0.5 - 6.0);
                let tip_cy = (btn.screen_center.y - btn.radius_px - 16.0)
                    .clamp(rect.min.y + pill_h * 0.5 + 4.0, rect.max.y - pill_h * 0.5 - 4.0);
                let pill_rect = Rect::from_center_size(
                    Pos2::new(tip_cx, tip_cy),
                    EguiVec2::new(pill_w, pill_h),
                );
                painter.rect_filled(
                    pill_rect,
                    6.0,
                    Color32::from_rgba_unmultiplied(15, 23, 42, 235),
                );
                painter.rect_stroke(
                    pill_rect,
                    6.0,
                    Stroke::new(1.1_f32, Color32::from_rgb(56, 189, 248)),
                    egui::StrokeKind::Inside,
                );
                painter.galley(
                    Pos2::new(
                        pill_rect.center().x - galley.size().x * 0.5,
                        pill_rect.center().y - galley.size().y * 0.5,
                    ),
                    galley,
                    Color32::WHITE,
                );
            }
        }
    }

    let sidebar_drag_active = dragging_catalog.is_some() || dragging_model_template.is_some();
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
        measure_locked,
        snapping_enabled,
        layer_protection,
        board_edit_enabled,
        selection_scope,
        sidebar_drag_active,
        measure_mode,
        terrain_brush,
        is_de,
        next_piece_seq,
        active_board_id,
        dragging_board,
        board_snap_guides,
        sketcher,
        dragging_model_rot,
        dragging_track_rot,
        ramp_overlay,
        &active_layer_id,
        &board_header_rects,
        &plus_buttons,
        &accessory_buttons,
        selected_flex_handles,
        selected_model_rot_handle,
        selected_track_rot_handle,
        rotation_step,
        &mut interaction,
    );

    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("railbench_is_de"), is_de));
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
        dragging_model_template,
        collab_users,
        dark_mode,
        show_rulers,
        snapping_enabled,
        measure_mode,
        sketcher.active,
        &mut interaction,
    );

    interaction
}
