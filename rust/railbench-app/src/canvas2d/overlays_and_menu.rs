use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_overlays_and_context_menu(
    ui: &mut egui::Ui,
    rect: Rect,
    response: &egui::Response,
    painter: &egui::Painter,
    layout: &Layout,
    cache: &Canvas2DCache,
    active_layer_id: &str,
    catalogs: &[Catalog],
    camera: &Camera2D,
    selected: &HashSet<String>,
    measure_points: &Option<(Vec2, Vec2)>,
    dragging_catalog: Option<&CatalogEntry>,
    dragging_model_template: Option<&Model3D>,
    collab_users: &[crate::collab::CollabUser],
    dark_mode: bool,
    show_rulers: bool,
    snapping_enabled: bool,
    measure_mode: bool,
    rotation_step: f64,
    interaction: &mut CanvasInteraction,
) {
    // 7. Measure tool overlay
    if measure_mode {
        if let Some((a, b)) = *measure_points {
            let sa = camera.world_to_screen(rect, a.x, a.y);
            let sb = camera.world_to_screen(rect, b.x, b.y);
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let dist_mm = dx.hypot(dy);
            let angle_deg = dy.atan2(dx).to_degrees();
            let col = Color32::from_rgb(245, 158, 11);
            // Dashed delta guides (right triangle legs)
            if dx.abs() > 1.0 && dy.abs() > 1.0 {
                let sc = camera.world_to_screen(rect, b.x, a.y);
                let guide_col = Color32::from_rgba_unmultiplied(245, 158, 11, 110);
                painter.add(egui::Shape::dashed_line(
                    &[sa, sc],
                    Stroke::new(1.1_f32, guide_col),
                    5.0,
                    4.0,
                ));
                painter.add(egui::Shape::dashed_line(
                    &[sc, sb],
                    Stroke::new(1.1_f32, guide_col),
                    5.0,
                    4.0,
                ));
            }
            painter.line_segment([sa, sb], Stroke::new(2.2_f32, col));
            painter.circle_filled(sa, 4.5, col);
            painter.circle_stroke(sa, 7.0, Stroke::new(1.2_f32, Color32::WHITE));
            painter.circle_filled(sb, 4.5, col);
            painter.circle_stroke(sb, 7.0, Stroke::new(1.2_f32, Color32::WHITE));
            let mid = Pos2::new((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5 - 16.0);
            let lbl = format!("{dist_mm:.1} mm ({:.1} cm) · {angle_deg:.1}°", dist_mm / 10.0);
            let bg_w = (lbl.len() as f32 * 6.5 + 18.0).max(165.0);
            let bg_r = Rect::from_center_size(mid, EguiVec2::new(bg_w, 22.0));
            painter.rect_filled(bg_r, 6.0, Color32::from_rgba_unmultiplied(15, 23, 42, 235));
            painter.rect_stroke(bg_r, 6.0, Stroke::new(1.1_f32, col), egui::StrokeKind::Inside);
            painter.text(mid, egui::Align2::CENTER_CENTER, lbl, FontId::proportional(11.5), Color32::WHITE);
        }
    }

    // 7b. Remote Collaborator Selection Highlights & Cursors
    for user in collab_users {
        let ucol = parse_hex_color(Some(&user.color)).unwrap_or(Color32::from_rgb(59, 130, 246));
        for sel_id in &user.selected_ids {
            if let Some(&idx) = cache.piece_id_to_idx.get(sel_id.as_str()) {
                if let Some(geom) = cache.pieces.get(idx) {
                    let smin = camera.world_to_screen(rect, geom.min_x - 8.0, geom.min_y - 8.0);
                    let smax = camera.world_to_screen(rect, geom.max_x + 8.0, geom.max_y + 8.0);
                    let r = Rect::from_two_pos(smin, smax);
                    if r.intersects(rect) {
                        painter.rect_stroke(r, 5.0, Stroke::new(2.0_f32, ucol), egui::StrokeKind::Outside);
                        let badge_pos = Pos2::new(r.max.x, r.min.y - 8.0);
                        let bw = (user.name.len() as f32 * 6.0 + 10.0).max(38.0);
                        let br = Rect::from_center_size(badge_pos, EguiVec2::new(bw, 15.0));
                        painter.rect_filled(br, 4.0, ucol);
                        painter.text(
                            br.center(),
                            egui::Align2::CENTER_CENTER,
                            &user.name,
                            FontId::proportional(9.5),
                            Color32::WHITE,
                        );
                    }
                }
            } else if let Some(m) = layout.models.as_ref().and_then(|ms| ms.iter().find(|m| m.id == *sel_id)) {
                let sc = m.scale.unwrap_or(1.0);
                let hw = m.width * sc * 0.5 + 8.0;
                let hd = m.depth * sc * 0.5 + 8.0;
                let smin = camera.world_to_screen(rect, m.transform.x - hw, m.transform.y - hd);
                let smax = camera.world_to_screen(rect, m.transform.x + hw, m.transform.y + hd);
                let r = Rect::from_two_pos(smin, smax);
                if r.intersects(rect) {
                    painter.rect_stroke(r, 5.0, Stroke::new(2.0_f32, ucol), egui::StrokeKind::Outside);
                }
            }
        }
        if let Some([wx, wy]) = user.cursor {
            let sp = camera.world_to_screen(rect, wx, wy);
            if rect.expand(36.0).contains(sp) {
                let tip = sp;
                let pts = vec![
                    tip,
                    tip + EguiVec2::new(0.0, 14.0),
                    tip + EguiVec2::new(4.5, 10.5),
                    tip + EguiVec2::new(11.0, 10.5),
                ];
                painter.add(egui::Shape::convex_polygon(
                    pts,
                    ucol,
                    Stroke::new(1.3_f32, Color32::from_rgb(15, 23, 42)),
                ));
                let tag_pos = tip + EguiVec2::new(14.0, 15.0);
                let tw = (user.name.len() as f32 * 6.2 + 12.0).max(44.0);
                let tr = Rect::from_min_size(tag_pos, EguiVec2::new(tw, 16.0));
                painter.rect_filled(tr, 4.0, ucol);
                painter.text(
                    tr.center(),
                    egui::Align2::CENTER_CENTER,
                    &user.name,
                    FontId::proportional(10.0),
                    Color32::WHITE,
                );
            }
        }
    }

    // 7.4. Model Drag-and-Drop live footprint ghost preview + drop handling
    if let Some(m) = dragging_model_template {
        if let Some(pointer) = ui.input(|i| i.pointer.latest_pos()) {
            if rect.contains(pointer) {
                let wp = camera.screen_to_world(rect, pointer);
                let sc = m.scale.unwrap_or(1.0);
                let hw = (m.width * sc * 0.5).max(12.0);
                let hd = (m.depth * sc * 0.5).max(12.0);
                let rot_r = m.transform.rotation.to_radians();
                let (cr, sr) = (rot_r.cos(), rot_r.sin());
                let corners = [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)];
                let pts: Vec<Pos2> = corners
                    .iter()
                    .map(|&(lx, ly)| {
                        camera.world_to_screen(rect, wp.x + lx * cr - ly * sr, wp.y + lx * sr + ly * cr)
                    })
                    .collect();
                painter.add(egui::Shape::convex_polygon(
                    pts,
                    Color32::from_rgba_unmultiplied(16, 185, 129, 48),
                    Stroke::new(1.8_f32, Color32::from_rgb(16, 185, 129)),
                ));
                let center_s = camera.world_to_screen(rect, wp.x, wp.y);
                painter.text(
                    center_s,
                    egui::Align2::CENTER_CENTER,
                    &m.name,
                    FontId::proportional(11.0),
                    Color32::WHITE,
                );
                if ui.input(|i| i.pointer.primary_released()) {
                    interaction.drop_model_at = Some(wp);
                }
            }
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

                // Draw live ghost track / turntable / transfer table at cursor
                let ghost_stroke_col = Color32::from_rgba_unmultiplied(56, 189, 248, 170);
                let ghost_fill_col = Color32::from_rgba_unmultiplied(56, 189, 248, 36);
                let ghost_pit_col = Color32::from_rgba_unmultiplied(15, 23, 42, 95);
                if entry.kind == "turntable" {
                    let outer_r = crate::turntables_3d::turntable_outer_radius(&ghost, Some(entry));
                    let scale_f = (outer_r / 189.0).clamp(0.4, 2.5);
                    let pit_r = 139.0 * scale_f;
                    let ring_rail_r = 124.0 * scale_f;
                    let gauge = entry.gauge.unwrap_or(16.5);
                    let hg = gauge * 0.5;
                    let girder_z = (hg + 4.2).max(12.6);
                    let center = camera.world_to_screen(rect, ghost.transform.x, ghost.transform.y);
                    let to_g = |lx: f64, ly: f64| -> Pos2 {
                        camera.world_to_screen(rect, ghost.transform.x + lx, ghost.transform.y + ly)
                    };
                    painter.circle_filled(center, (outer_r as f32) * camera.zoom, ghost_fill_col);
                    painter.circle_stroke(center, (outer_r as f32) * camera.zoom, Stroke::new(1.8_f32, ghost_stroke_col));
                    painter.circle_filled(center, (pit_r as f32) * camera.zoom, ghost_pit_col);
                    painter.circle_stroke(center, (pit_r as f32) * camera.zoom, Stroke::new(1.4_f32, ghost_stroke_col));
                    painter.circle_stroke(center, (ring_rail_r as f32) * camera.zoom, Stroke::new(1.0_f32, ghost_stroke_col));

                    let hb = pit_r - 1.0;
                    let bridge_pts = vec![
                        to_g(-hb, -girder_z),
                        to_g(hb, -girder_z),
                        to_g(hb, girder_z),
                        to_g(-hb, girder_z),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        bridge_pts,
                        Color32::from_rgba_unmultiplied(56, 189, 248, 70),
                        Stroke::new(1.4_f32, ghost_stroke_col),
                    ));
                    let cab_cx = -16.0 * scale_f;
                    let cab_hw = 17.0 * scale_f;
                    let cab_z0 = girder_z + 7.7;
                    let cab_z1 = cab_z0 + 20.5 * scale_f;
                    let cab_pts = vec![
                        to_g(cab_cx - cab_hw, cab_z0),
                        to_g(cab_cx + cab_hw, cab_z0),
                        to_g(cab_cx + cab_hw, cab_z1),
                        to_g(cab_cx - cab_hw, cab_z1),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        cab_pts,
                        ghost_fill_col,
                        Stroke::new(1.2_f32, ghost_stroke_col),
                    ));
                    for side in [-1.0_f64, 1.0] {
                        painter.line_segment(
                            [to_g(-hb, side * hg), to_g(hb, side * hg)],
                            Stroke::new(1.4_f32, ghost_stroke_col),
                        );
                    }
                    let slot_angles: Vec<f64> = entry
                        .local_connectors
                        .as_ref()
                        .filter(|lc| !lc.is_empty())
                        .map(|lc| lc.iter().map(|c| c.angle).collect())
                        .unwrap_or_else(|| (0..30).map(|i| i as f64 * 12.0).collect());
                    for deg in slot_angles {
                        let rad = deg.to_radians();
                        let (ca, sa) = (rad.cos(), rad.sin());
                        painter.line_segment(
                            [to_g(pit_r * ca, pit_r * sa), to_g(outer_r * ca, outer_r * sa)],
                            Stroke::new(1.2_f32, ghost_stroke_col),
                        );
                    }
                } else if entry.kind == "transfer-table" {
                    let (_, l, top_z, bot_z) = crate::turntables_3d::transfer_table_local_bounds(Some(entry));
                    let pit_left = 36.0;
                    let pit_right = l - 36.0;
                    let pit_top = top_z + 6.0;
                    let pit_bot = bot_z - 6.0;
                    let track_zs = crate::turntables_3d::extract_transfer_track_zs(Some(entry));
                    let bridge_cx = l * 0.5;
                    let bridge_z = track_zs[track_zs.len() / 2] as f64;
                    let gauge = entry.gauge.unwrap_or(16.5);
                    let hg = gauge * 0.5;
                    let girder_z = (hg + 4.2).max(12.6);
                    let to_g = |lx: f64, ly: f64| -> Pos2 {
                        camera.world_to_screen(rect, ghost.transform.x + lx, ghost.transform.y + ly)
                    };
                    let rect_pts = |x0: f64, x1: f64, y0: f64, y1: f64| -> Vec<Pos2> {
                        vec![to_g(x0, y0), to_g(x1, y0), to_g(x1, y1), to_g(x0, y1)]
                    };
                    painter.add(egui::Shape::convex_polygon(
                        rect_pts(0.0, l, top_z, bot_z),
                        ghost_fill_col,
                        Stroke::new(1.8_f32, ghost_stroke_col),
                    ));
                    painter.add(egui::Shape::convex_polygon(
                        rect_pts(pit_left, pit_right, pit_top, pit_bot),
                        ghost_pit_col,
                        Stroke::new(1.4_f32, ghost_stroke_col),
                    ));
                    for rx in [pit_left + 44.0, l * 0.5, pit_right - 44.0] {
                        painter.line_segment(
                            [to_g(rx, pit_top + 6.0), to_g(rx, pit_bot - 6.0)],
                            Stroke::new(1.1_f32, ghost_stroke_col),
                        );
                    }
                    painter.add(egui::Shape::convex_polygon(
                        rect_pts(pit_left + 1.0, pit_right - 1.0, bridge_z - girder_z, bridge_z + girder_z),
                        Color32::from_rgba_unmultiplied(56, 189, 248, 70),
                        Stroke::new(1.4_f32, ghost_stroke_col),
                    ));
                    let cab_cx = bridge_cx - 34.0;
                    let cab_z0 = bridge_z + girder_z + 7.7;
                    painter.add(egui::Shape::convex_polygon(
                        rect_pts(cab_cx - 18.0, cab_cx + 18.0, cab_z0, cab_z0 + 21.0),
                        ghost_fill_col,
                        Stroke::new(1.2_f32, ghost_stroke_col),
                    ));
                    for side in [-1.0_f64, 1.0] {
                        painter.line_segment(
                            [
                                to_g(pit_left + 1.0, bridge_z + side * hg),
                                to_g(pit_right - 1.0, bridge_z + side * hg),
                            ],
                            Stroke::new(1.4_f32, ghost_stroke_col),
                        );
                    }
                    for &tzf in &track_zs {
                        let tz = tzf as f64;
                        for side in [-1.0_f64, 1.0] {
                            painter.line_segment(
                                [to_g(0.0, tz + side * hg), to_g(pit_left, tz + side * hg)],
                                Stroke::new(1.2_f32, ghost_stroke_col),
                            );
                            painter.line_segment(
                                [to_g(pit_right, tz + side * hg), to_g(l, tz + side * hg)],
                                Stroke::new(1.2_f32, ghost_stroke_col),
                            );
                        }
                    }
                } else {
                    for branch in piece_branches_world(&ghost, Some(entry)) {
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
                }

                // Live snap target ring if near an open connector of the SAME type (track-to-track or road-to-road)
                if snapping_enabled {
                    let ghost_is_road = entry.is_road == Some(true) || crate::catalog::is_road_id(catalogs, &entry.id);
                    let snap_radius_sq = snap_radius * snap_radius;
                    let active_lh = layout
                        .layers
                        .as_ref()
                        .and_then(|ls| ls.iter().find(|l| l.id == active_layer_id))
                        .and_then(|l| l.height_mm)
                        .unwrap_or(0.0);
                    let mut best_same: Option<(f64, f64, f64, f64, f64)> = None;
                    let mut best_diff: Option<(f64, f64, f64, f64, f64)> = None;
                    for c1 in &ghost.connectors {
                        for other in &layout.pieces {
                            let vis = other
                                .layer_id
                                .as_deref()
                                .and_then(|lid| cache.layer_flags.get(lid))
                                .map_or_else(
                                    || is_layer_visible(layout, other.layer_id.as_deref()),
                                    |&(v, _)| v,
                                );
                            if !vis {
                                continue;
                            }
                            if crate::catalog::is_road_id(catalogs, &other.catalog_id) != ghost_is_road {
                                continue;
                            }
                            let other_lh = other
                                .layer_id
                                .as_deref()
                                .and_then(|lid| {
                                    layout
                                        .layers
                                        .as_ref()
                                        .and_then(|ls| ls.iter().find(|l| l.id == lid))
                                        .and_then(|l| l.height_mm)
                                })
                                .unwrap_or(0.0);
                            for (c2_idx, c2) in other.connectors.iter().enumerate() {
                                if c2.connected_to.is_some() {
                                    continue;
                                }
                                let dx = c1.x - c2.x;
                                let dy = c1.y - c2.y;
                                let d_sq = dx * dx + dy * dy;
                                if d_sq <= snap_radius_sq {
                                    let h2 = railbench_core::snapping::piece_connector_height(other, c2_idx, other_lh);
                                    if (active_lh - h2).abs() <= 1.0 {
                                        if best_same.as_ref().map_or(true, |b| d_sq < b.4) {
                                            best_same = Some((c1.x, c1.y, c2.x, c2.y, d_sq));
                                        }
                                    } else if best_diff.as_ref().map_or(true, |b| d_sq < b.4) {
                                        best_diff = Some((c1.x, c1.y, c2.x, c2.y, d_sq));
                                    }
                                }
                            }
                        }
                        if !ghost_is_road {
                            if let Some(models) = &layout.models {
                                for m in models {
                                    if !is_layer_visible(layout, m.layer_id.as_deref()) {
                                        continue;
                                    }
                                    let m_h = crate::building_snap::model_world_height(layout, m);
                                    for door in crate::building_snap::get_building_door_connectors(m) {
                                        let dx = c1.x - door.x;
                                        let dy = c1.y - door.y;
                                        let d_sq = dx * dx + dy * dy;
                                        if d_sq <= snap_radius_sq {
                                            if (active_lh - m_h).abs() <= 1.0 {
                                                if best_same.as_ref().map_or(true, |b| d_sq < b.4) {
                                                    best_same = Some((c1.x, c1.y, door.x, door.y, d_sq));
                                                }
                                            } else if best_diff.as_ref().map_or(true, |b| d_sq < b.4) {
                                                best_diff = Some((c1.x, c1.y, door.x, door.y, d_sq));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some((sx, sy, dx, dy, _)) = best_same {
                        let p1 = camera.world_to_screen(rect, sx, sy);
                        let p2 = camera.world_to_screen(rect, dx, dy);
                        let cyan = Color32::from_rgb(56, 189, 248);
                        painter.line_segment([p1, p2], Stroke::new(1.8_f32, cyan));
                        painter.circle_stroke(p2, 10.0, Stroke::new(2.2_f32, cyan));
                        painter.circle_filled(p2, 4.0, Color32::from_rgb(34, 197, 94));
                    } else if let Some((sx, sy, dx, dy, _)) = best_diff {
                        let p1 = camera.world_to_screen(rect, sx, sy);
                        let p2 = camera.world_to_screen(rect, dx, dy);
                        let yellow = Color32::from_rgb(250, 204, 21);
                        painter.line_segment([p1, p2], Stroke::new(1.8_f32, yellow));
                        painter.circle_stroke(p2, 10.5, Stroke::new(2.4_f32, yellow));
                        painter.circle_filled(p2, 5.0, Color32::from_rgb(234, 179, 8));
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
        let sel_pieces: Vec<&TrackPiece> = layout
            .pieces
            .iter()
            .filter(|p| selected.contains(&p.id))
            .collect();
        let any_track_sel = !sel_pieces.is_empty();
        let first_piece = sel_pieces.first().copied();
        let first_entry = first_piece.and_then(|p| find_entry(catalogs, &p.catalog_id));
        let any_turnout_sel = sel_pieces.iter().any(|p| {
            find_entry(catalogs, &p.catalog_id).map_or(false, |e| crate::is_turnout_kind(&e.kind))
        });
        let any_flex_sel = sel_pieces
            .iter()
            .any(|p| crate::catalog::is_flex_id(catalogs, &p.catalog_id));
        let all_grouped = sel_pieces.len() >= 2
            && first_piece.and_then(|p| p.group_id.as_ref()).is_some()
            && sel_pieces.iter().all(|p| p.group_id == first_piece.and_then(|f| f.group_id.clone()));
        let any_signal_sel = sel_pieces.iter().any(|p| p.has_signal == Some(true));
        let ctx_sig_side = sel_pieces
            .iter()
            .find(|p| p.has_signal == Some(true))
            .and_then(|p| p.signal_side.as_deref())
            .unwrap_or("right")
            .to_string();
        let ctx_sig_dir = sel_pieces
            .iter()
            .find(|p| p.has_signal == Some(true))
            .and_then(|p| p.signal_direction.as_deref())
            .unwrap_or("forward")
            .to_string();
        let ctx_layers: Vec<(String, String, f64)> = layout
            .layers
            .as_ref()
            .map(|ls| {
                ls.iter()
                    .map(|l| (l.id.clone(), l.name.clone(), l.height_mm.unwrap_or(0.0)))
                    .collect()
            })
            .unwrap_or_else(|| vec![("layer-1".into(), "Ebene 0".into(), 0.0)]);
        let is_de = ui.ctx().data(|d| {
            d.get_temp::<bool>(egui::Id::new("railbench_is_de"))
                .unwrap_or(true)
        });

        response.context_menu(|ui| {
            ui.set_min_width(220.0);
            if let Some(fp) = first_piece {
                let title = if sel_pieces.len() == 1 {
                    let name = first_entry
                        .map(|e| if !is_de { e.name_en.as_deref().unwrap_or(&e.name) } else { e.name.as_str() })
                        .unwrap_or(&fp.catalog_id);
                    format!("#{} · {}", fp.catalog_id, name)
                } else if is_de {
                    format!("{} Elemente ausgewaehlt", selected.len())
                } else {
                    format!("{} items selected", selected.len())
                };
                ui.label(egui::RichText::new(title).strong().size(12.0));
                if sel_pieces.len() == 1 {
                    let open_cnt = fp.connectors.iter().filter(|c| c.connected_to.is_none()).count();
                    let lid = fp.layer_id.as_deref().unwrap_or("layer-1");
                    let (lname, lh) = ctx_layers
                        .iter()
                        .find(|(id, _, _)| id == lid)
                        .map(|(_, n, h)| (n.as_str(), *h))
                        .unwrap_or(("Ebene 0", 0.0));
                    let rel_z = fp.elevation.map_or(0.0, |e| e.start);
                    let meta = if is_de {
                        format!("{lname} ({:.0} mm) · {open_cnt}/{} offen", lh + rel_z, fp.connectors.len())
                    } else {
                        format!("{lname} ({:.0} mm) · {open_cnt}/{} open", lh + rel_z, fp.connectors.len())
                    };
                    ui.small(meta);
                }
                ui.separator();
            }

            if ui.button(if is_de { format!("↺ Drehen -{rotation_step}°") } else { format!("↺ Rotate -{rotation_step}°") }).clicked() {
                interaction.rotate_delta = Some(-rotation_step);
                ui.close_menu();
            }
            if ui.button(if is_de { format!("↻ Drehen +{rotation_step}°") } else { format!("↻ Rotate +{rotation_step}°") }).clicked() {
                interaction.rotate_delta = Some(rotation_step);
                ui.close_menu();
            }
            if any_track_sel
                && ui.button(if is_de { "⇄ Spiegeln / Wenden (Alt+F)" } else { "⇄ Mirror / Flip (Alt+F)" }).clicked()
            {
                interaction.flip_selected = true;
                ui.close_menu();
            }
            if ui.button(if is_de { "Kopieren (Strg+C)" } else { "Copy (Ctrl+C)" }).clicked() {
                interaction.copy_selected = true;
                ui.close_menu();
            }
            if ui.button(if is_de { "Einfuegen (Strg+V)" } else { "Paste (Ctrl+V)" }).clicked() {
                interaction.paste_clipboard = true;
                ui.close_menu();
            }
            if ui.button(if is_de { "Duplizieren (Strg+D)" } else { "Duplicate (Ctrl+D)" }).clicked() {
                interaction.duplicate_selected = true;
                ui.close_menu();
            }
            if sel_pieces.len() >= 2 {
                let grp_lbl = if all_grouped {
                    if is_de { "Gruppierung aufheben (G)" } else { "Ungroup Tracks (G)" }
                } else if is_de {
                    "Gleise gruppieren (G)"
                } else {
                    "Group Tracks (G)"
                };
                if ui.button(grp_lbl).clicked() {
                    interaction.toggle_group = true;
                    ui.close_menu();
                }
            }
            ui.menu_button(if is_de { "Auf Ebene verschieben" } else { "Move to Layer" }, |ui| {
                for (lid, lname, _) in &ctx_layers {
                    if ui.button(lname).clicked() {
                        interaction.move_to_layer = Some(lid.clone());
                        ui.close_menu();
                    }
                }
            });
            if any_track_sel {
                ui.separator();
                if ui.button(if is_de { "📐 Hoehe & Steigung... (M)" } else { "📐 Height & Ramp... (M)" }).clicked() {
                    interaction.open_height_dialog = true;
                    ui.close_menu();
                }
                if ui.button(if is_de { "⚡ Auto-Rampe anwenden (Shift+M)" } else { "⚡ Apply Auto-Ramp (Shift+M)" }).clicked() {
                    interaction.auto_ramp = true;
                    ui.close_menu();
                }
                if ui.button(if is_de { "🔗 Verbundene Gleise auswaehlen" } else { "🔗 Select Connected Tracks" }).clicked() {
                    interaction.select_connected = true;
                    ui.close_menu();
                }
                if ui.button(if is_de { "📑 Ganze Ebene auswaehlen" } else { "📑 Select Whole Layer" }).clicked() {
                    interaction.select_layer = true;
                    ui.close_menu();
                }
                if let Some(fp) = first_piece {
                    if sel_pieces.len() == 1 {
                        if ui.button(if is_de { "✂ Gleis teilen / ersetzen..." } else { "✂ Split / Replace Track..." }).clicked() {
                            interaction.open_replace_split = Some(fp.id.clone());
                            ui.close_menu();
                        }
                        if any_flex_sel
                            && ui.button(if is_de { "↺ Flexgleis zuruecksetzen" } else { "↺ Reset Flextrack" }).clicked()
                        {
                            interaction.reset_flex = Some(fp.id.clone());
                            ui.close_menu();
                        }
                        if ui.button(if is_de { "ℹ Eigenschaften / Info..." } else { "ℹ Properties / Info..." }).clicked() {
                            interaction.open_track_props = Some(fp.id.clone());
                            ui.close_menu();
                        }
                    }
                }
                ui.separator();
                if ui.button(if is_de { "⚡ Stromeinspeisung (F)" } else { "⚡ Toggle Power Feeder (F)" }).clicked() {
                    interaction.toggle_feeder = true;
                    ui.close_menu();
                }
                if ui.button(if is_de { "⊘ Isolierverbinder (I)" } else { "⊘ Toggle Isolator (I)" }).clicked() {
                    interaction.toggle_isolator = true;
                    ui.close_menu();
                }
                if ui.button(if is_de { "⌁ Oberleitung (O)" } else { "⌁ Toggle Catenary (O)" }).clicked() {
                    interaction.toggle_catenary = true;
                    ui.close_menu();
                }
                if ui.button(if is_de { "⚡ Oberleitungs-Speisung (Shift+F)" } else { "⚡ Catenary Feeder (Shift+F)" }).clicked() {
                    interaction.toggle_catenary_feeder = true;
                    ui.close_menu();
                }
                if ui.button(if is_de { "⊘ Oberleitungs-Trenner (Shift+I)" } else { "⊘ Catenary Isolator (Shift+I)" }).clicked() {
                    interaction.toggle_catenary_isolator = true;
                    ui.close_menu();
                }
                if ui.button(if is_de { "🚦 Signal (Shift+S)" } else { "🚦 Toggle Signal (Shift+S)" }).clicked() {
                    interaction.toggle_signal = true;
                    ui.close_menu();
                }
                if any_signal_sel {
                    if ui.button(if is_de { format!("Signal-Seite: {ctx_sig_side}") } else { format!("Signal Side: {ctx_sig_side}") }).clicked() {
                        interaction.cycle_signal_side = true;
                        ui.close_menu();
                    }
                    if ui.button(if is_de { format!("Signal-Richtung: {ctx_sig_dir}") } else { format!("Signal Direction: {ctx_sig_dir}") }).clicked() {
                        interaction.cycle_signal_direction = true;
                        ui.close_menu();
                    }
                }
                if any_turnout_sel {
                    if ui.button(if is_de { "Weichenlaterne (W)" } else { "Turnout Lantern (W)" }).clicked() {
                        interaction.toggle_lantern = true;
                        ui.close_menu();
                    }
                    if ui.button(if is_de { "Weichendecoder (Shift+W)" } else { "Turnout Decoder (Shift+W)" }).clicked() {
                        interaction.toggle_decoder = true;
                        ui.close_menu();
                    }
                }
                if ui.button(if is_de { "🌉 Bruecke (B)" } else { "🌉 Toggle Bridge (B)" }).clicked() {
                    interaction.toggle_bridge = true;
                    ui.close_menu();
                }
                if ui.button(if is_de { "🚇 Tunnel (T)" } else { "🚇 Toggle Tunnel (T)" }).clicked() {
                    interaction.toggle_tunnel = true;
                    ui.close_menu();
                }
            }
            ui.separator();
            if ui.button(if is_de { "Loeschen (Entf)" } else { "Delete Selected (Del)" }).clicked() {
                interaction.delete_selected = true;
                ui.close_menu();
            }
        });
    }

}
