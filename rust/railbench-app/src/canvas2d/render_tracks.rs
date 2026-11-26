use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_track_piece(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    piece: &TrackPiece,
    branches: &[Vec<Vec2>],
    all_pieces: &[TrackPiece],
    piece_id_to_idx: &HashMap<String, usize>,
    entry: Option<&CatalogEntry>,
    special_top_mesh: Option<&CachedModelTopMesh2D>,
    is_selected: bool,
    is_active_layer: bool,
    is_editable: bool,
    color_mode: ColorMode,
    layer_color_hex: Option<&str>,
    show_labels: bool,
    power_sim_enabled: bool,
    piece_power: Option<&PiecePowerResult>,
    turnout_state: usize,
    turntable_angle_deg: f64,
    transfer_offset_mm: Option<f64>,
    uncoupler_state: Option<(bool, bool)>,
    accessory_buttons: &mut Vec<TrackAccessoryButton>,
) {
    let kind = entry.map(|e| e.kind.as_str()).unwrap_or("straight");
    let is_special = matches!(kind, "turntable" | "transfer-table");
    if !is_special && (branches.is_empty() || branches[0].is_empty()) {
        return;
    }
    let is_road = entry.and_then(|e| e.is_road).unwrap_or(false);
    let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5);
    let road_w_mm = entry.and_then(|e| e.road_width).unwrap_or(68.0);
    let ballast_w = if is_road {
        (road_w_mm * camera.zoom as f64).max(5.0) as f32
    } else {
        ((gauge / 16.5) * 34.0 * camera.zoom as f64).max(3.5) as f32
    };
    let rail_w = (1.6 * camera.zoom).max(1.0);

    let bp = crate::blueprint_colors::get_track_blueprint_color(&piece.catalog_id);
    let mut ballast_col = if is_road {
        Color32::from_rgb(55, 65, 81)
    } else if piece_power.is_some_and(|pr| pr.is_short_circuit) {
        Color32::from_rgb(239, 68, 68)
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
            ColorMode::Off => Color32::from_rgb(0x8a, 0x84, 0x79),
            ColorMode::Layers => {
                parse_hex_color(layer_color_hex).unwrap_or(Color32::from_rgb(0x38, 0xbd, 0xf8))
            }
            ColorMode::Types => bp.color32(),
        }
    };
    let (mut tie_col, mut rail_col) = if is_road || piece_power.is_some_and(|pr| pr.is_short_circuit) || power_sim_enabled {
        (Color32::from_rgb(0x3d, 0x27, 0x1d), Color32::from_rgb(0xc8, 0xd0, 0xd8))
    } else {
        match color_mode {
            ColorMode::Types | ColorMode::Layers => (
                Color32::from_rgb(0x1e, 0x29, 0x3b),
                Color32::from_rgb(0xf8, 0xfa, 0xfc),
            ),
            ColorMode::Off => (
                Color32::from_rgb(0x3d, 0x27, 0x1d),
                Color32::from_rgb(0xc8, 0xd0, 0xd8),
            ),
        }
    };

    if !is_active_layer && !is_selected {
        // Inactive layers are desaturated & faded matching Electron track-sprite.ts (0.72, alpha 0.58)
        ballast_col = crate::blueprint_colors::desaturate_electron_color32(ballast_col, 0.72, 0.58);
        tie_col = crate::blueprint_colors::desaturate_electron_color32(tie_col, 0.72, 0.58);
        rail_col = crate::blueprint_colors::desaturate_electron_color32(rail_col, 0.72, 0.58);
    } else if !is_selected {
        // Unselected active-layer tracks darken by 30% so selected tracks keep 100% saturation
        ballast_col = crate::blueprint_colors::darken_color32(ballast_col, 0.30);
        tie_col = crate::blueprint_colors::darken_color32(tie_col, 0.30);
        rail_col = crate::blueprint_colors::darken_color32(rail_col, 0.30);
    }
    if piece.has_tunnel == Some(true) {
        ballast_col = Color32::from_rgba_unmultiplied(ballast_col.r(), ballast_col.g(), ballast_col.b(), 120);
    }

    if kind == "turntable" {
        draw_turntable_2d(
            painter,
            rect,
            camera,
            piece,
            entry,
            special_top_mesh,
            gauge,
            rail_w,
            rail_col,
            ballast_col,
            ballast_w,
            color_mode,
            is_selected,
            is_active_layer,
            is_editable,
            show_labels,
            power_sim_enabled,
            piece_power,
            turntable_angle_deg,
            accessory_buttons,
        );
        return;
    }
    if kind == "transfer-table" {
        draw_transfer_table_2d(
            painter,
            rect,
            camera,
            piece,
            all_pieces,
            piece_id_to_idx,
            entry,
            special_top_mesh,
            gauge,
            rail_w,
            rail_col,
            ballast_col,
            ballast_w,
            color_mode,
            is_selected,
            is_active_layer,
            is_editable,
            show_labels,
            power_sim_enabled,
            piece_power,
            transfer_offset_mm,
            accessory_buttons,
        );
        return;
    }
    let half_g = gauge * 0.5;
    let half_tie = gauge * 0.72;
    let tie_w = (2.2 * camera.zoom).max(1.0);

    // Pass 1: Selection glow + Ballast roadbed for ALL branches first
    // (Continuous mitered polygon strip + 0.35 mm endpoint tangent extension so curved segments and track joints have zero cuts)
    let mut screen_branches: Vec<Vec<Pos2>> = if is_road {
        Vec::with_capacity(branches.len())
    } else {
        Vec::new()
    };
    let ballast_half_mm = if is_road {
        road_w_mm * 0.5
    } else {
        (gauge / 16.5) * 17.0
    };
    for pts in branches {
        if pts.len() < 2 {
            if is_road {
                screen_branches.push(Vec::new());
            }
            continue;
        }
        let screen_pts: Vec<Pos2> = pts
            .iter()
            .map(|p| camera.world_to_screen(rect, p.x, p.y))
            .collect();
        if is_selected {
            draw_mitered_ribbon_2d(
                painter,
                rect,
                camera,
                pts,
                ballast_half_mm + (3.5 / camera.zoom.max(0.1)) as f64,
                0.45,
                Color32::from_rgba_unmultiplied(56, 189, 248, 110),
            );
        }
        draw_mitered_ribbon_2d(painter, rect, camera, pts, ballast_half_mm, 0.35, ballast_col);
        if is_road {
            screen_branches.push(screen_pts);
        }
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
        // Pass 2: Bridge girders + Sleepers / ties across ALL branches (LOD skips sub-2px sleepers)
        let tie_step = (gauge / 16.5) * 9.5;
        let draw_sleepers = (tie_step as f32) * camera.zoom >= 2.0;
        let has_bridge = piece.has_bridge == Some(true);
        if has_bridge || draw_sleepers {
            for pts in branches {
                if pts.len() < 2 {
                    continue;
                }
                if has_bridge {
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

                if draw_sleepers {
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
            }
        }

        // Pass 3: Twin metallic rails on top of ALL ballast & sleepers (mitered with 0.30 mm tangent overlap at ends)
        for pts in branches {
            if pts.len() < 2 {
                continue;
            }
            let n = pts.len();
            let mut left_rail = Vec::with_capacity(n);
            let mut right_rail = Vec::with_capacity(n);
            for i in 0..n {
                let (nx, ny, mscale) = polyline_miter_normal_2d(pts, i);
                let tx = ny;
                let ty = -nx;
                let pad = if i == 0 {
                    -0.30
                } else if i + 1 == n {
                    0.30
                } else {
                    0.0
                };
                let px = pts[i].x + tx * pad;
                let py = pts[i].y + ty * pad;
                let hg = half_g * mscale;
                left_rail.push(camera.world_to_screen(rect, px + nx * hg, py + ny * hg));
                right_rail.push(camera.world_to_screen(rect, px - nx * hg, py - ny * hg));
            }
            painter.add(egui::Shape::line(left_rail, Stroke::new(rail_w, rail_col)));
            painter.add(egui::Shape::line(right_rail, Stroke::new(rail_w, rail_col)));
        }

        // Pass 3.1: Maerklin 24997 / Catalog Uncoupler Ramp (Entkupplerbohle) between rails
        if entry.is_some_and(crate::catalog::is_uncoupler_entry) {
            if let Some(pts) = branches.first().filter(|b| b.len() >= 2) {
                let mid_i = pts.len() / 2;
                let a = pts[mid_i.saturating_sub(1)];
                let b = pts[(mid_i + 1).min(pts.len() - 1)];
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let len = dx.hypot(dy).max(1e-6);
                let ux = dx / len;
                let uy = dy / len;
                let nx = -uy;
                let ny = ux;
                let cx = pts[mid_i].x;
                let cy = pts[mid_i].y;
                let hl = 19.0 * (gauge / 16.5);
                let hw = 4.6 * (gauge / 16.5);
                let to_scr = |along: f64, lat: f64| -> Pos2 {
                    camera.world_to_screen(rect, cx + ux * along + nx * lat, cy + uy * along + ny * lat)
                };
                let (raised, armed) = uncoupler_state.unwrap_or((false, false));
                let ramp_fill = if raised {
                    Color32::from_rgb(245, 158, 11)
                } else if armed {
                    Color32::from_rgb(139, 92, 246)
                } else {
                    Color32::from_rgb(71, 85, 105)
                };
                let ramp_stroke = if raised || armed {
                    Color32::from_rgb(254, 240, 138)
                } else {
                    Color32::from_rgb(203, 213, 225)
                };
                painter.add(egui::Shape::convex_polygon(
                    vec![to_scr(-hl, -hw), to_scr(hl, -hw), to_scr(hl, hw), to_scr(-hl, hw)],
                    ramp_fill,
                    Stroke::new((1.2 * camera.zoom).clamp(0.9, 2.0), ramp_stroke),
                ));
            }
        }

        // Pass 3.2: 2D Prellbock (Buffer Stop: steel A-frame struts, Pufferbohle, twin Pufferteller & red/white Sh2 sign)
        let is_buffer = piece.connectors.len() == 1 || entry.map_or(false, crate::catalog::is_buffer_stop_entry);
        if is_buffer {
            if let Some(pts) = branches.first().filter(|b| b.len() >= 2) {
                let p_end = pts[pts.len() - 1];
                let p_prev = pts[pts.len() - 2];
                let dx = p_end.x - p_prev.x;
                let dy = p_end.y - p_prev.y;
                let len = dx.hypot(dy).max(1e-6);
                let ux = dx / len;
                let uy = dy / len;
                let nx = -uy;
                let ny = ux;

                // Helper to transform local (along_end, lat) relative to p_end into screen Pos2
                let to_scr = |along: f64, lat: f64| -> Pos2 {
                    camera.world_to_screen(
                        rect,
                        p_end.x + ux * along + nx * lat,
                        p_end.y + uy * along + ny * lat,
                    )
                };
                let poly = |along0: f64, along1: f64, lat0: f64, lat1: f64| -> Vec<Pos2> {
                    vec![
                        to_scr(along0, lat0),
                        to_scr(along1, lat0),
                        to_scr(along1, lat1),
                        to_scr(along0, lat1),
                    ]
                };

                let sc = (gauge / 16.5).max(0.4);
                let bohle_x0 = -7.5 * sc;
                let bohle_x1 = -3.0 * sc;
                let bohle_half_h = half_g + 4.5 * sc;

                // Steel A-frame struts clamping onto both rails behind the buffer beam
                let strut_w = (1.8 * camera.zoom).clamp(1.0, 3.2);
                let strut_col = Color32::from_rgb(71, 85, 105);
                for side in [-1.0_f64, 1.0] {
                    let rl = side * half_g;
                    painter.line_segment([to_scr(-16.0 * sc, rl), to_scr(bohle_x0, rl * 0.92)], Stroke::new(strut_w, strut_col));
                    painter.line_segment([to_scr(-0.8 * sc, rl), to_scr(bohle_x1, rl * 0.92)], Stroke::new(strut_w, strut_col));
                }

                // Buffer plates (twin Pufferteller facing approaching train towards -ux)
                let p_off = half_g * 0.78;
                let p_half = 2.3 * sc;
                for side in [-1.0_f64, 1.0] {
                    let cy = side * p_off;
                    let plate_pts = poly(bohle_x0 - 2.6 * sc, bohle_x0, cy - p_half, cy + p_half);
                    painter.add(egui::Shape::convex_polygon(
                        plate_pts,
                        Color32::from_rgb(30, 41, 59),
                        Stroke::new((0.8 * camera.zoom).max(0.6), Color32::from_rgb(15, 23, 42)),
                    ));
                }

                // Main buffer beam (Pufferbohle)
                let beam_pts = poly(bohle_x0, bohle_x1, -bohle_half_h, bohle_half_h);
                painter.add(egui::Shape::convex_polygon(
                    beam_pts,
                    Color32::from_rgb(107, 62, 30),
                    Stroke::new((1.0 * camera.zoom).max(0.8), Color32::from_rgb(15, 23, 42)),
                ));

                // Sh 2 protection halt sign (white border + red center rectangle)
                let sh2_outer = poly(bohle_x0 - 0.4 * sc, bohle_x0 + 2.2 * sc, -3.2 * sc, 3.2 * sc);
                painter.add(egui::Shape::convex_polygon(
                    sh2_outer,
                    Color32::from_rgb(248, 250, 252),
                    Stroke::new((0.6 * camera.zoom).max(0.5), Color32::from_rgb(15, 23, 42)),
                ));
                let sh2_inner = poly(bohle_x0, bohle_x0 + 1.8 * sc, -2.2 * sc, 2.2 * sc);
                painter.add(egui::Shape::convex_polygon(
                    sh2_inner,
                    Color32::from_rgb(239, 68, 68),
                    Stroke::NONE,
                ));
            }
        }
    }

    // Pass 3.5: 2D Tunnel corridor, dashed walls, open-end stone portal brackets & arch badge
    if piece.has_tunnel == Some(true) && piece.connectors.len() < 6 {
        let tunnel_half_w = if is_road {
            road_w_mm * 0.5 + 6.0
        } else if piece.has_catenary == Some(true) {
            (gauge * 1.65).max(26.0)
        } else {
            (gauge * 1.25).max(20.0)
        };
        let wall_outer_w = (3.0 * camera.zoom).clamp(1.2, 3.2);
        let wall_dash_w = (1.8 * camera.zoom).clamp(0.8, 2.2);
        let corridor_w = (tunnel_half_w as f32 * 2.0 * camera.zoom).max(4.0);
        let wall_outer_col = Color32::from_rgba_unmultiplied(51, 65, 85, 220);
        let wall_dash_col = Color32::from_rgba_unmultiplied(203, 213, 225, 230);
        let draw_dashes = 17.0 * camera.zoom >= 3.0;

        for pts in branches {
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

                if draw_dashes {
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
        }

        let neighbor_has_tunnel = |c_idx: usize| -> bool {
            piece
                .connectors
                .get(c_idx)
                .and_then(|c| c.connected_to.as_ref())
                .and_then(|link| {
                    piece_id_to_idx
                        .get(&link.piece_id)
                        .and_then(|&idx| all_pieces.get(idx))
                })
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
                let br = Rect::from_center_size(Pos2::new(sp.x, sp.y + 13.0), EguiVec2::new(18.0, 14.0));
                painter.rect_filled(br, 3.5, Color32::from_rgba_unmultiplied(30, 41, 59, 235));
                painter.rect_stroke(br, 3.5, Stroke::new(1.0_f32, Color32::from_rgb(148, 163, 184)), egui::StrokeKind::Inside);
                let c = br.center();
                let icon_s = Stroke::new(1.15_f32, Color32::from_rgb(248, 250, 252));
                // Vector tunnel arch symbol (outer portal arch, inner arch, and ground baseline)
                let mut outer_pts = Vec::with_capacity(9);
                outer_pts.push(Pos2::new(c.x - 4.5, c.y + 4.2));
                for k in 0..=6 {
                    let a = std::f32::consts::PI * (1.0 - k as f32 / 6.0);
                    outer_pts.push(Pos2::new(c.x + a.cos() * 4.5, c.y - 0.2 - a.sin() * 4.2));
                }
                outer_pts.push(Pos2::new(c.x + 4.5, c.y + 4.2));
                painter.add(egui::Shape::line(outer_pts, icon_s));
                let mut inner_pts = Vec::with_capacity(7);
                inner_pts.push(Pos2::new(c.x - 2.3, c.y + 4.2));
                for k in 0..=4 {
                    let a = std::f32::consts::PI * (1.0 - k as f32 / 4.0);
                    inner_pts.push(Pos2::new(c.x + a.cos() * 2.3, c.y + 0.8 - a.sin() * 2.3));
                }
                inner_pts.push(Pos2::new(c.x + 2.3, c.y + 4.2));
                painter.add(egui::Shape::line(inner_pts, icon_s));
                painter.line_segment([Pos2::new(c.x - 5.5, c.y + 4.2), Pos2::new(c.x + 5.5, c.y + 4.2)], icon_s);
            }
        }
    }

    // Pass 4: Overhead Catenary Wire & Masts
    if piece.has_catenary == Some(true) && !is_road {
        let cat_col = catenary_color_2d(power_sim_enabled, piece_power, is_active_layer, is_selected);
        let wire_w = (1.8 * camera.zoom).clamp(1.2, 2.8);
        let mast_col = Color32::from_rgb(51, 65, 85);
        for (br_idx, pts) in branches.iter().enumerate() {
            if pts.len() < 2 {
                continue;
            }
            let spts: Vec<Pos2> = pts
                .iter()
                .map(|p| camera.world_to_screen(rect, p.x, p.y))
                .collect();
            painter.add(egui::Shape::line(spts, Stroke::new(wire_w, cat_col)));

            let mast_off = (gauge * 1.15).max(20.0);
            let side_sign = match kind {
                "curve" => {
                    if entry.and_then(|e| e.angle).unwrap_or(30.0) < 0.0 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                "turnout-left" | "curved-turnout-left" => {
                    if br_idx == 0 { 1.0 } else { -1.0 }
                }
                "turnout-right" | "curved-turnout-right" => {
                    if br_idx == 0 { -1.0 } else { 1.0 }
                }
                _ => {
                    if br_idx % 2 == 0 { 1.0 } else { -1.0 }
                }
            };
            let mut end_indices = Vec::with_capacity(2);
            if br_idx == 0 {
                end_indices.push(0usize);
            }
            if piece.connectors.len() > 1 {
                end_indices.push(pts.len() - 1);
            }
            for idx in end_indices {
                let (nx, ny, _) = polyline_miter_normal_2d(pts, idx);
                let mp = pts[idx];
                let mx = mp.x + nx * mast_off * side_sign;
                let my = mp.y + ny * mast_off * side_sign;
                let sm = camera.world_to_screen(rect, mx, my);
                let sc = camera.world_to_screen(rect, mp.x, mp.y);
                painter.line_segment([sm, sc], Stroke::new((1.4 * camera.zoom).max(1.0), mast_col));
                painter.circle_filled(sm, (2.4 * camera.zoom).clamp(1.8, 4.0), mast_col);
            }
        }
    }

    // Article number label, power simulation readouts & feeder/isolator badges
    let main_pts = &branches[0];
    let mid_w = main_pts[main_pts.len() / 2];
    draw_piece_label_and_badges(
        painter,
        rect,
        camera,
        piece,
        entry,
        mid_w,
        ballast_w,
        gauge,
        is_road,
        is_selected,
        is_active_layer,
        is_editable,
        show_labels,
        power_sim_enabled,
        piece_power,
        uncoupler_state,
        accessory_buttons,
    );

    // Turnout lantern (DB Wn 1 / Wn 2 Weichenlaterne) & digital decoder overlays
    let is_turnout = crate::is_turnout_kind(kind);
    let show_lantern_housing = is_turnout || piece.has_lantern == Some(true);
    let lantern_lit = piece.has_lantern == Some(true);
    let active_branch = turnout_state;
    let is_left_turnout = kind == "turnout-left" || kind == "curved-turnout-left";
    let is_right_turnout = kind == "turnout-right" || kind == "curved-turnout-right";
    let acc_side_y = if is_left_turnout { -22.0 } else { 22.0 };
    if show_lantern_housing || piece.has_decoder == Some(true) {
        let root_r = rotate(Vec2 { x: 26.0, y: 0.0 }, piece.transform.rotation);
        let l_r = rotate(Vec2 { x: 26.0, y: acc_side_y }, piece.transform.rotation);
        let s_root = camera.world_to_screen(rect, piece.transform.x + root_r.x, piece.transform.y + root_r.y);
        let s_lant = camera.world_to_screen(rect, piece.transform.x + l_r.x, piece.transform.y + l_r.y);
        painter.line_segment([s_root, s_lant], Stroke::new((1.6 * camera.zoom).clamp(1.0, 2.4), Color32::from_rgb(100, 116, 139)));

        if show_lantern_housing {
            let sz = (8.8 * camera.zoom).clamp(6.5, 13.0);
            let r = Rect::from_center_size(s_lant, EguiVec2::splat(sz));
            if lantern_lit {
                // Warm glow halo + illuminated ring only when lantern is equipped/lit
                painter.circle_filled(s_lant, sz * 0.85, Color32::from_rgba_unmultiplied(254, 240, 138, 42));
                painter.circle_stroke(s_lant, sz * 0.72, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(254, 240, 138, 135)));
            }
            // Black DB lantern housing with metallic border
            painter.rect_filled(r, 2.0, Color32::from_rgb(11, 17, 32));
            painter.rect_stroke(r, 2.0, Stroke::new(1.1_f32, Color32::from_rgb(100, 116, 139)), egui::StrokeKind::Inside);

            // DB Wn 1 / Wn 2 position bar: bright warm milk-glass when lit, matte unlit white-grey otherwise
            let aspect_col = if lantern_lit {
                Color32::from_rgb(254, 249, 195)
            } else {
                Color32::from_rgb(186, 198, 214)
            };
            let bar_len = sz * 0.34;
            let bar_w = (sz * 0.20).clamp(1.4, 2.6);
            let branch_offset = if kind == "double-slip" {
                if active_branch % 2 == 0 {
                    0.0
                } else {
                    std::f32::consts::FRAC_PI_4
                }
            } else if active_branch == 0 {
                0.0
            } else if is_left_turnout || (kind == "three-way" && active_branch == 1) {
                -std::f32::consts::FRAC_PI_4
            } else if is_right_turnout || kind == "three-way" {
                std::f32::consts::FRAC_PI_4
            } else {
                0.0
            };
            let sym_ang = (piece.transform.rotation as f32).to_radians() + branch_offset;
            let dx = sym_ang.cos() * bar_len;
            let dy = sym_ang.sin() * bar_len;
            painter.line_segment(
                [Pos2::new(s_lant.x - dx, s_lant.y - dy), Pos2::new(s_lant.x + dx, s_lant.y + dy)],
                Stroke::new(bar_w, aspect_col),
            );
        }

        if piece.has_decoder == Some(true) {
            let dec_x_local = if show_lantern_housing { 38.0 } else { 26.0 };
            let d_r = rotate(Vec2 { x: dec_x_local, y: acc_side_y }, piece.transform.rotation);
            let s_dec = camera.world_to_screen(rect, piece.transform.x + d_r.x, piece.transform.y + d_r.y);
            let dr = Rect::from_center_size(s_dec, EguiVec2::new(22.0, 11.0));
            painter.rect_filled(dr, 2.5, Color32::from_rgb(15, 23, 42));
            painter.rect_stroke(dr, 2.5, Stroke::new(1.1_f32, Color32::from_rgb(2, 132, 199)), egui::StrokeKind::Inside);
            painter.text(s_dec, egui::Align2::CENTER_CENTER, "DEC", FontId::monospace(7.0), Color32::from_rgb(56, 189, 248));
        }
    }

    // Trackside DB Lichthauptsignal(s) in 2D
    if piece.has_signal == Some(true) && !is_road && main_pts.len() >= 2 {
        let lat_dist = gauge * 0.95 + 7.5;
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
                let s_mast = camera.world_to_screen(rect, sx, sy);

                // Mast arm & white-red-white DB Mastschild marker at mast foot
                painter.line_segment([s_base, s_mast], Stroke::new((1.5 * camera.zoom).clamp(1.1, 2.2), Color32::from_rgb(100, 116, 139)));
                let mast_r = (2.2 * camera.zoom).clamp(1.6, 3.2);
                painter.circle_filled(s_mast, mast_r, Color32::from_rgb(248, 250, 252));
                painter.circle_filled(s_mast, mast_r * 0.55, Color32::from_rgb(220, 38, 38));

                // 3-aspect DB signal head shield oriented along track direction (Green / Red / Yellow)
                let fwd_sign = if is_fwd { 1.0 } else { -1.0 };
                let head_cx = sx + ux * 6.0 * fwd_sign;
                let head_cy = sy + uy * 6.0 * fwd_sign;
                let half_l = 7.2_f64;
                let half_w = 3.4_f64;
                let to_s = |al: f64, la: f64| -> Pos2 {
                    camera.world_to_screen(
                        rect,
                        head_cx + ux * al * fwd_sign + nx * la,
                        head_cy + uy * al * fwd_sign + ny * la,
                    )
                };
                let shield_poly = vec![
                    to_s(-half_l, -half_w),
                    to_s(half_l, -half_w),
                    to_s(half_l, half_w),
                    to_s(-half_l, half_w),
                ];
                painter.add(egui::Shape::convex_polygon(
                    shield_poly,
                    Color32::from_rgb(15, 23, 42),
                    Stroke::new((1.0 * camera.zoom).clamp(0.8, 1.5), Color32::from_rgb(248, 250, 252)),
                ));

                let lens_r = (2.1 * camera.zoom).clamp(1.6, 3.2);
                let s_grn = to_s(4.2, 0.0);
                let s_red = to_s(0.0, 0.0);
                let s_yel = to_s(-4.2, 0.0);
                painter.circle_filled(s_grn, lens_r, Color32::from_rgb(34, 197, 94));
                painter.circle_filled(s_red, lens_r, Color32::from_rgb(239, 68, 68));
                painter.circle_filled(s_yel, lens_r * 0.88, Color32::from_rgb(245, 158, 11));
            }
        }
    }
}

pub(super) fn polyline_miter_normal_2d(pts: &[Vec2], i: usize) -> (f64, f64, f64) {
    let seg_norm = |a: Vec2, b: Vec2| -> (f64, f64) {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = dx.hypot(dy).max(1e-6);
        (-dy / len, dx / len)
    };
    let n = pts.len();
    if n < 2 {
        return (0.0, 1.0, 1.0);
    }
    if i == 0 {
        let (nx, ny) = seg_norm(pts[0], pts[1]);
        return (nx, ny, 1.0);
    }
    if i >= n - 1 {
        let (nx, ny) = seg_norm(pts[n - 2], pts[n - 1]);
        return (nx, ny, 1.0);
    }
    let (n0x, n0y) = seg_norm(pts[i - 1], pts[i]);
    let (n1x, n1y) = seg_norm(pts[i], pts[i + 1]);
    let mx = n0x + n1x;
    let my = n0y + n1y;
    let mlen = mx.hypot(my);
    if mlen < 1e-5 {
        return (n1x, n1y, 1.0);
    }
    let ux = mx / mlen;
    let uy = my / mlen;
    let cos_half = (ux * n1x + uy * n1y).clamp(0.45, 1.0);
    (ux, uy, 1.0 / cos_half)
}

