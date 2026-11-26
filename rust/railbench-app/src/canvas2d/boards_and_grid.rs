use super::*;

pub(super) fn draw_grid(painter: &egui::Painter, rect: Rect, camera: &Camera2D, dark_mode: bool) {
    let top_left = camera.screen_to_world(rect, rect.min);
    let bot_right = camera.screen_to_world(rect, rect.max);

    let fine_color = if dark_mode {
        Color32::from_rgba_unmultiplied(148, 163, 184, 18)
    } else {
        Color32::from_rgba_unmultiplied(102, 92, 78, 22)
    };
    let major_color = if dark_mode {
        Color32::from_rgba_unmultiplied(148, 163, 184, 42)
    } else {
        Color32::from_rgba_unmultiplied(102, 92, 78, 50)
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

pub(super) fn board_style_colors(style: &str) -> (Color32, Color32, Color32) {
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

pub(super) fn draw_cached_tris_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    tris: &[[Vec2; 3]],
    outer: &[Vec2],
    holes: &[Vec<Vec2>],
    fill: Color32,
    outer_stroke: Option<Stroke>,
    hole_stroke: Option<Stroke>,
) {
    if !tris.is_empty() {
        let mut mesh = egui::Mesh::default();
        mesh.vertices.reserve(tris.len() * 3);
        mesh.indices.reserve(tris.len() * 3);
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

pub(super) fn fill_concave_polygon_2d(
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
    draw_cached_tris_2d(painter, rect, camera, &tris, outer, holes, fill, outer_stroke, hole_stroke);
}

pub(super) fn poly_centroid(poly: &[Vec2]) -> Vec2 {
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

pub(super) fn find_board_drag_hit(
    board_edit_enabled: bool,
    layout: &Layout,
    wp: Vec2,
    sp: Pos2,
    zoom: f32,
    header_rects: &[(String, Rect)],
    allow_interior: bool,
) -> Option<(String, BoardBounds)> {
    if !board_edit_enabled {
        return None;
    }
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
pub(super) fn draw_baseboards(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    cached_boards: &[CachedBaseboard2D],
    active_board_id: Option<&str>,
    snap_guides: &[BaseboardSnapGuide],
    dark_mode: bool,
    board_edit_enabled: bool,
) -> Vec<(String, Rect)> {
    let mut header_rects = Vec::new();
    let multi = cached_boards.len() > 1;
    let first_id = cached_boards.first().map(|b| b.id.as_str());

    for cb in cached_boards {
        let bid = cb.id.as_str();
        let is_active = active_board_id.map_or_else(|| Some(bid) == first_id, |aid| aid == bid);
        let (fill, border_col, grain_col) = board_style_colors(&cb.style);

        if board_edit_enabled && multi && is_active {
            let pts: Vec<Pos2> = cb
                .mesh
                .outer
                .iter()
                .map(|p| camera.world_to_screen(rect, p.x, p.y))
                .collect();
            painter.add(egui::Shape::closed_line(
                pts,
                Stroke::new(6.0_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 85)),
            ));
        }

        let border_stroke = if board_edit_enabled && multi && is_active {
            Stroke::new(2.2_f32, Color32::from_rgb(56, 189, 248))
        } else {
            Stroke::new(2.0_f32, border_col)
        };
        draw_cached_tris_2d(
            painter,
            rect,
            camera,
            &cb.mesh.tris,
            &cb.mesh.outer,
            &cb.mesh.holes,
            fill,
            Some(border_stroke),
            None,
        );

        // Subtle inner inset edge for wood craft look
        if multi || cb.style != "white" {
            let _ = grain_col;
        }

        // Draw cutout recess wells + amber outline
        for (hi, h_tris) in cb.hole_tris.iter().enumerate() {
            draw_cached_tris_2d(
                painter,
                rect,
                camera,
                h_tris,
                &cb.mesh.holes[hi],
                &[],
                Color32::from_rgba_unmultiplied(15, 23, 42, 175),
                Some(Stroke::new(1.4_f32, Color32::from_rgba_unmultiplied(245, 158, 11, 200))),
                None,
            );
        }

        // Header badge (for clicking/dragging/identifying boards)
        if multi || cb.name.is_some() || camera.zoom >= 0.12 {
            let sp = camera.world_to_screen(rect, cb.anchor.x, cb.anchor.y);
            let title = cb.name.as_deref().unwrap_or("Baseboard");
            let pill_w = (title.len() as f32 * 6.5 + 22.0).clamp(58.0, 170.0);
            let hrect = Rect::from_min_size(Pos2::new(sp.x + 6.0, sp.y + 6.0), EguiVec2::new(pill_w, 18.0));
            let pill_bg = if board_edit_enabled && is_active {
                Color32::from_rgba_unmultiplied(14, 116, 144, 225)
            } else {
                Color32::from_rgba_unmultiplied(30, 41, 59, 175)
            };
            painter.rect_filled(hrect, 4.0, pill_bg);
            painter.rect_stroke(
                hrect,
                4.0,
                Stroke::new(
                    1.0_f32,
                    if board_edit_enabled && is_active {
                        Color32::from_rgb(56, 189, 248)
                    } else {
                        border_col
                    },
                ),
                egui::StrokeKind::Inside,
            );
            painter.text(
                hrect.center(),
                egui::Align2::CENTER_CENTER,
                title,
                FontId::proportional(10.0),
                Color32::WHITE,
            );
            if board_edit_enabled {
                header_rects.push((cb.id.clone(), hrect));
            }
        }

        // Dimension lines
        if cb.show_dimensions != Some(false) && camera.zoom >= 0.08 {
            let bounds = cb.bounds;
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
pub(super) fn draw_layer_wood_plates(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    layout: &Layout,
    cache: &Canvas2DCache,
    catalogs: &[Catalog],
    layer_id: &str,
    is_active_layer: bool,
    layer_color_hex: Option<&str>,
    view_aabb: (f64, f64, f64, f64),
) {
    let Some(layer) = layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == layer_id))
    else {
        return;
    };
    let accent = parse_hex_color(layer_color_hex)
        .unwrap_or(Color32::from_rgb(56, 189, 248));
    let alpha_fill = if is_active_layer { 175 } else { 75 };
    let alpha_stroke = if is_active_layer { 220 } else { 110 };

    if let Some(plates) = cache.layer_plates.get(layer_id) {
        if !plates.is_empty() {
            for cp in plates {
                let (base_fill, _, _) = board_style_colors(&cp.style);
                let fill = Color32::from_rgba_unmultiplied(
                    base_fill.r(),
                    base_fill.g(),
                    base_fill.b(),
                    if cp.is_table_mode {
                        if is_active_layer { 145 } else { 60 }
                    } else {
                        alpha_fill
                    },
                );
                let stroke = Stroke::new(
                    if cp.is_table_mode { 1.4_f32 } else { 1.6_f32 },
                    Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), alpha_stroke),
                );
                draw_cached_tris_2d(
                    painter,
                    rect,
                    camera,
                    &cp.mesh.tris,
                    &cp.mesh.outer,
                    &cp.mesh.holes,
                    fill,
                    Some(stroke),
                    Some(Stroke::new(1.2_f32, Color32::from_rgb(245, 158, 11))),
                );
                if let Some(name) = &cp.name {
                    if camera.zoom >= 0.14 {
                        let sc = camera.world_to_screen(rect, cp.centroid.x, cp.centroid.y);
                        if rect.expand(48.0).contains(sc) {
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
            }
            return;
        }
    }

    let mode = layer.wood_plate_mode.as_deref().unwrap_or("none");
    if mode == "tight" && !cache.baseboards.is_empty() {
        let (base_fill, _, _) =
            board_style_colors(layer.wood_plate_style.as_deref().unwrap_or("plywood"));
        let roadbed_col = Color32::from_rgba_unmultiplied(
            base_fill.r(),
            base_fill.g(),
            base_fill.b(),
            if is_active_layer { 150 } else { 65 },
        );
        let w_scr = (72.0 * camera.zoom).max(6.0);
        if let Some(p_indices) = cache.layer_piece_indices.get(layer_id) {
            for &pi in p_indices {
                let pg = &cache.pieces[pi];
                if !aabb_intersects(
                    (pg.min_x, pg.min_y, pg.max_x, pg.max_y),
                    view_aabb.0,
                    view_aabb.1,
                    view_aabb.2,
                    view_aabb.3,
                ) {
                    continue;
                }
                let p = &layout.pieces[pi];
                let entry = find_entry(catalogs, &p.catalog_id);
                if let Some(e) = entry {
                    if e.kind == "turntable" {
                        let outer_r = crate::turntables_3d::turntable_outer_radius(p, entry);
                        let sc = camera.world_to_screen(rect, p.transform.x, p.transform.y);
                        painter.circle_filled(sc, ((outer_r + 12.0) as f32) * camera.zoom, roadbed_col);
                        continue;
                    }
                    if e.kind == "transfer-table" {
                        let (lx0, lx1, ly0, ly1) = crate::turntables_3d::transfer_table_local_bounds(entry);
                        let pad = 12.0;
                        let spts = [
                            Vec2 { x: lx0 - pad, y: ly0 - pad },
                            Vec2 { x: lx1 + pad, y: ly0 - pad },
                            Vec2 { x: lx1 + pad, y: ly1 + pad },
                            Vec2 { x: lx0 - pad, y: ly1 + pad },
                        ]
                        .into_iter()
                        .map(|lc| {
                            let rp = rotate(lc, p.transform.rotation);
                            camera.world_to_screen(rect, p.transform.x + rp.x, p.transform.y + rp.y)
                        })
                        .collect();
                        painter.add(egui::Shape::convex_polygon(spts, roadbed_col, Stroke::NONE));
                        continue;
                    }
                }
                if p.connectors.len() < 2 {
                    continue;
                }
                let p0 = camera.world_to_screen(rect, p.connectors[0].x, p.connectors[0].y);
                let p1 = camera.world_to_screen(rect, p.connectors[1].x, p.connectors[1].y);
                painter.line_segment([p0, p1], Stroke::new(w_scr, roadbed_col));
            }
        }
    }
}

pub(super) fn draw_sketcher_overlay(
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

pub(super) fn catenary_color_2d(
    power_sim_enabled: bool,
    piece_power: Option<&PiecePowerResult>,
    is_active_layer: bool,
    is_selected: bool,
) -> Color32 {
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
    cat_col
}

pub(super) fn draw_rulers(painter: &egui::Painter, rect: Rect, camera: &Camera2D, dark_mode: bool) {
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

pub(super) fn brighten_color(c: Color32, amount: f32) -> Color32 {
    let t = amount.clamp(0.0, 1.0);
    let r = (c.r() as f32 + (255.0 - c.r() as f32) * t).round() as u8;
    let g = (c.g() as f32 + (255.0 - c.g() as f32) * t).round() as u8;
    let b = (c.b() as f32 + (255.0 - c.b() as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(r, g, b, c.a())
}

pub(super) fn desaturate_color(c: Color32, sat_drop: f32, alpha_scale: f32) -> Color32 {
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

