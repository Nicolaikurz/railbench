use super::*;

pub(super) fn draw_projected_mesh_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    tm: &CachedModelTopMesh2D,
    tr: Transform,
    is_selected: bool,
    is_active_layer: bool,
    is_tunnel: bool,
) {
    if tm.tris.is_empty() {
        return;
    }
    let rad = tr.rotation.to_radians();
    let (sin_r, cos_r) = rad.sin_cos();
    let mut mesh = egui::Mesh::default();
    mesh.vertices.reserve(tm.tris.len() * 3);
    mesh.indices.reserve(tm.tris.len() * 3);

    for (pts, cols) in &tm.tris {
        let base = mesh.vertices.len() as u32;
        for k in 0..3 {
            let lx = pts[k][0] as f64;
            let ly = pts[k][1] as f64;
            let wx = tr.x + lx * cos_r - ly * sin_r;
            let wy = tr.y + lx * sin_r + ly * cos_r;
            let mut col = cols[k];
            if !is_active_layer && !is_selected {
                col = desaturate_color(col, 0.78, 0.72);
            } else if is_selected {
                col = brighten_color(col, 0.14);
            }
            if is_tunnel {
                col = Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 135);
            }
            mesh.colored_vertex(camera.world_to_screen(rect, wx, wy), col);
        }
        mesh.add_triangle(base, base + 1, base + 2);
    }
    painter.add(egui::Shape::mesh(mesh));
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_piece_label_and_badges(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    label_world: Vec2,
    ballast_w: f32,
    gauge: f64,
    is_road: bool,
    is_selected: bool,
    is_active_layer: bool,
    is_editable: bool,
    show_labels: bool,
    power_sim_enabled: bool,
    piece_power: Option<&PiecePowerResult>,
    uncoupler_state: Option<(bool, bool)>,
    accessory_buttons: &mut Vec<TrackAccessoryButton>,
) {
    let mid = camera.world_to_screen(rect, label_world.x, label_world.y);
    if show_labels && (is_selected || ballast_w >= 5.5) && rect.expand(48.0).contains(mid) {
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
        let galley = painter.layout_no_wrap(label_text, FontId::proportional(font_sz), txt_col);
        let pill_h = (galley.size().y + pad_y * 2.0).clamp(min_h, max_h);
        let pill_rect = Rect::from_center_size(mid, EguiVec2::new(galley.size().x + pad_x * 2.0, pill_h));
        let bp = crate::blueprint_colors::get_track_blueprint_color(&piece.catalog_id);
        let mut border_col = if is_road {
            Color32::from_rgb(148, 163, 184)
        } else {
            bp.border_color32()
        };
        if !is_active_layer && !is_selected {
            border_col = crate::blueprint_colors::desaturate_electron_color32(border_col, 0.72, 0.58);
        }
        painter.rect_filled(
            pill_rect,
            (pill_h * 0.22).clamp(2.0, 5.0),
            Color32::from_rgba_unmultiplied(15, 23, 42, bg_alpha),
        );
        painter.rect_stroke(
            pill_rect,
            (pill_h * 0.22).clamp(2.0, 5.0),
            Stroke::new(0.9_f32, border_col),
            egui::StrokeKind::Inside,
        );
        let text_pos = Pos2::new(
            pill_rect.center().x - galley.size().x * 0.5,
            pill_rect.center().y - galley.size().y * 0.5,
        );
        painter.galley(text_pos, galley, txt_col);
    }

    let has_roadbed = entry.and_then(|e| e.has_roadbed).unwrap_or(true);
    let ballast_half_mm = if has_roadbed {
        if gauge >= 30.0 {
            gauge * 1.09
        } else if gauge >= 16.0 {
            18.0
        } else {
            gauge * 1.1
        }
    } else {
        gauge * 0.75
    };
    let stem_start_mm = (ballast_half_mm - 6.0).max(12.0);
    let badge_offset_mm = (ballast_half_mm + 8.0).max(26.0);
    let badge_r_px = (11.0 * camera.zoom).clamp(6.5, 16.0);
    let ring_w = (2.0 * camera.zoom).clamp(1.4, 2.8);
    let stem_w = (2.0 * camera.zoom).clamp(1.2, 3.0);
    let rot_rad = piece.transform.rotation.to_radians();
    let (nx, ny) = (-rot_rad.sin(), rot_rad.cos());

    let feeder_active = piece.power_feed == Some(true);
    let feeder_grey = !feeder_active && entry.is_some_and(crate::catalog::is_feeder_entry);
    if (feeder_active || feeder_grey) && !is_road {
        let s_stem0 = camera.world_to_screen(
            rect,
            label_world.x + nx * stem_start_mm,
            label_world.y + ny * stem_start_mm,
        );
        let badge_c = camera.world_to_screen(
            rect,
            label_world.x + nx * badge_offset_mm,
            label_world.y + ny * badge_offset_mm,
        );
        let dv = badge_c - s_stem0;
        let dlen = dv.length().max(1.0);
        let s_stem1 = badge_c - dv * ((badge_r_px - 0.5).min(dlen) / dlen);
        let stem_col = if feeder_active {
            Color32::from_rgba_unmultiplied(234, 179, 8, 230)
        } else {
            Color32::from_rgba_unmultiplied(148, 163, 184, 155)
        };
        painter.line_segment([s_stem0, s_stem1], Stroke::new(stem_w, stem_col));
        painter.circle_filled(
            badge_c + EguiVec2::new(1.0, 1.5),
            badge_r_px,
            Color32::from_rgba_unmultiplied(0, 0, 0, 95),
        );
        if feeder_active {
            painter.circle_filled(badge_c, badge_r_px, Color32::from_rgb(234, 179, 8));
            painter.circle_stroke(badge_c, badge_r_px, Stroke::new(ring_w, Color32::WHITE));
        } else {
            painter.circle_filled(
                badge_c,
                badge_r_px,
                Color32::from_rgba_unmultiplied(100, 116, 139, 195),
            );
            painter.circle_stroke(
                badge_c,
                badge_r_px,
                Stroke::new(
                    (1.5 * camera.zoom).clamp(1.0, 2.2),
                    Color32::from_rgb(148, 163, 184),
                ),
            );
        }
        let r = badge_r_px;
        let pt = |u: f32, v: f32| Pos2::new(badge_c.x + u * r, badge_c.y + v * r);
        let bolt_col = if feeder_active {
            Color32::from_rgb(254, 240, 138)
        } else {
            Color32::from_rgb(226, 232, 240)
        };
        let mut mesh = egui::Mesh::default();
        for tri in [
            [pt(0.12, -0.55), pt(-0.38, 0.08), pt(0.06, 0.08)],
            [pt(-0.06, -0.08), pt(0.38, -0.08), pt(-0.12, 0.55)],
        ] {
            let base = mesh.vertices.len() as u32;
            for p in tri {
                mesh.colored_vertex(p, bolt_col);
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        painter.add(egui::Shape::mesh(mesh));
        if is_editable {
            accessory_buttons.push(TrackAccessoryButton {
                piece_id: piece.id.clone(),
                kind: TrackAccessoryButtonKind::Feeder,
                screen_center: badge_c,
                radius_px: badge_r_px,
                active: feeder_active,
            });
        }
    }

    let iso_active = piece.is_isolator == Some(true);
    let iso_grey = !iso_active && entry.is_some_and(crate::catalog::is_isolating_entry);
    if (iso_active || iso_grey) && !is_road {
        let c_idx = piece.isolated_connector_index.unwrap_or(0);
        if let Some(c) = piece.connectors.get(c_idx).or_else(|| piece.connectors.first()) {
            let rad = c.angle.to_radians();
            let in_x = -rad.cos();
            let in_y = -rad.sin();
            let perp_x = -rad.sin();
            let perp_y = rad.cos();
            let cx = c.x + in_x * 5.0;
            let cy = c.y + in_y * 5.0;
            let cut_half = ballast_half_mm.max(18.0);
            let c0 = camera.world_to_screen(rect, cx - perp_x * cut_half, cy - perp_y * cut_half);
            let c1 = camera.world_to_screen(rect, cx + perp_x * cut_half, cy + perp_y * cut_half);
            if iso_active {
                painter.line_segment(
                    [c0, c1],
                    Stroke::new(
                        (4.0 * camera.zoom).clamp(2.2, 6.0),
                        Color32::from_rgba_unmultiplied(220, 38, 38, 245),
                    ),
                );
                let rx = cx + in_x * 1.5;
                let ry = cy + in_y * 1.5;
                let r0 = camera.world_to_screen(rect, rx - perp_x * cut_half, ry - perp_y * cut_half);
                let r1 = camera.world_to_screen(rect, rx + perp_x * cut_half, ry + perp_y * cut_half);
                painter.line_segment(
                    [r0, r1],
                    Stroke::new(
                        (1.5 * camera.zoom).clamp(1.0, 2.5),
                        Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                    ),
                );
            } else {
                painter.line_segment(
                    [c0, c1],
                    Stroke::new(
                        (3.0 * camera.zoom).clamp(1.8, 4.5),
                        Color32::from_rgba_unmultiplied(100, 116, 139, 160),
                    ),
                );
            }

            let side = if perp_x * nx + perp_y * ny >= 0.0 { 1.0 } else { -1.0 };
            let bx_dir = perp_x * side;
            let by_dir = perp_y * side;
            let s_stem0 = camera.world_to_screen(
                rect,
                cx + bx_dir * stem_start_mm,
                cy + by_dir * stem_start_mm,
            );
            let badge_c = camera.world_to_screen(
                rect,
                cx + bx_dir * badge_offset_mm,
                cy + by_dir * badge_offset_mm,
            );
            let dv = badge_c - s_stem0;
            let dlen = dv.length().max(1.0);
            let s_stem1 = badge_c - dv * ((badge_r_px - 0.5).min(dlen) / dlen);
            let stem_col = if iso_active {
                Color32::from_rgba_unmultiplied(220, 38, 38, 230)
            } else {
                Color32::from_rgba_unmultiplied(148, 163, 184, 155)
            };
            painter.line_segment([s_stem0, s_stem1], Stroke::new(stem_w, stem_col));
            painter.circle_filled(
                badge_c + EguiVec2::new(1.0, 1.5),
                badge_r_px,
                Color32::from_rgba_unmultiplied(0, 0, 0, 115),
            );
            if iso_active {
                painter.circle_filled(badge_c, badge_r_px, Color32::from_rgb(220, 38, 38));
                painter.circle_stroke(badge_c, badge_r_px, Stroke::new(ring_w, Color32::WHITE));
                let bar_r = Rect::from_center_size(
                    badge_c,
                    EguiVec2::new(badge_r_px * 1.12, (badge_r_px * 0.34).max(2.0)),
                );
                painter.rect_filled(bar_r, (badge_r_px * 0.14).max(1.0), Color32::WHITE);
            } else {
                painter.circle_filled(
                    badge_c,
                    badge_r_px,
                    Color32::from_rgba_unmultiplied(100, 116, 139, 195),
                );
                painter.circle_stroke(
                    badge_c,
                    badge_r_px,
                    Stroke::new(
                        (1.5 * camera.zoom).clamp(1.0, 2.2),
                        Color32::from_rgb(148, 163, 184),
                    ),
                );
                let sym_col = Color32::from_rgb(226, 232, 240);
                let inner_r = badge_r_px * 0.48;
                let slash_d = inner_r * std::f32::consts::FRAC_1_SQRT_2;
                painter.circle_stroke(badge_c, inner_r, Stroke::new(1.4_f32, sym_col));
                painter.line_segment(
                    [
                        Pos2::new(badge_c.x - slash_d, badge_c.y + slash_d),
                        Pos2::new(badge_c.x + slash_d, badge_c.y - slash_d),
                    ],
                    Stroke::new(1.4_f32, sym_col),
                );
            }
            if is_editable {
                accessory_buttons.push(TrackAccessoryButton {
                    piece_id: piece.id.clone(),
                    kind: TrackAccessoryButtonKind::Isolator,
                    screen_center: badge_c,
                    radius_px: badge_r_px,
                    active: iso_active,
                });
            }
        }
    }

    if !is_road && entry.is_some_and(crate::catalog::is_uncoupler_entry) {
        let (raised, armed) = uncoupler_state.unwrap_or((false, false));
        let active = raised || armed;
        let side_s = if piece.power_feed == Some(true) || entry.is_some_and(crate::catalog::is_feeder_entry) {
            -1.0_f64
        } else {
            1.0_f64
        };
        let ux = nx * side_s;
        let uy = ny * side_s;
        let s_stem0 = camera.world_to_screen(rect, label_world.x + ux * stem_start_mm, label_world.y + uy * stem_start_mm);
        let badge_c = camera.world_to_screen(rect, label_world.x + ux * badge_offset_mm, label_world.y + uy * badge_offset_mm);
        let dv = badge_c - s_stem0;
        let dlen = dv.length().max(1.0);
        let s_stem1 = badge_c - dv * ((badge_r_px - 0.5).min(dlen) / dlen);
        let stem_col = if raised {
            Color32::from_rgba_unmultiplied(245, 158, 11, 235)
        } else if armed {
            Color32::from_rgba_unmultiplied(139, 92, 246, 235)
        } else {
            Color32::from_rgba_unmultiplied(59, 130, 246, 175)
        };
        painter.line_segment([s_stem0, s_stem1], Stroke::new(stem_w, stem_col));
        painter.circle_filled(
            badge_c + EguiVec2::new(1.0, 1.5),
            badge_r_px,
            Color32::from_rgba_unmultiplied(0, 0, 0, 105),
        );
        let fill_col = if raised {
            Color32::from_rgb(217, 119, 6)
        } else if armed {
            Color32::from_rgb(124, 58, 237)
        } else {
            Color32::from_rgb(30, 64, 175)
        };
        let ring_col = if raised {
            Color32::from_rgb(254, 240, 138)
        } else {
            Color32::WHITE
        };
        painter.circle_filled(badge_c, badge_r_px, fill_col);
        painter.circle_stroke(badge_c, badge_r_px, Stroke::new(ring_w, ring_col));
        // Vector split-coupler icon: two separated jaws & central lift wedge
        let r = badge_r_px;
        let sym = Stroke::new((r * 0.19).clamp(1.2, 2.6), Color32::WHITE);
        painter.line_segment(
            [Pos2::new(badge_c.x - r * 0.48, badge_c.y), Pos2::new(badge_c.x - r * 0.16, badge_c.y)],
            sym,
        );
        painter.line_segment(
            [Pos2::new(badge_c.x + r * 0.16, badge_c.y), Pos2::new(badge_c.x + r * 0.48, badge_c.y)],
            sym,
        );
        painter.line_segment(
            [Pos2::new(badge_c.x, badge_c.y - r * 0.34), Pos2::new(badge_c.x, badge_c.y + r * 0.34)],
            Stroke::new((r * 0.16).clamp(1.1, 2.2), Color32::from_rgb(254, 240, 138)),
        );
        accessory_buttons.push(TrackAccessoryButton {
            piece_id: piece.id.clone(),
            kind: TrackAccessoryButtonKind::Uncoupler,
            screen_center: badge_c,
            radius_px: badge_r_px,
            active,
        });
    }

    if piece.catenary_power_feed == Some(true) && !is_road {
        let p = Pos2::new(mid.x - 13.0, mid.y - 11.0);
        let r = Rect::from_center_size(p, EguiVec2::new(20.0, 12.0));
        painter.rect_filled(r, 4.0, Color32::from_rgb(2, 132, 199));
        painter.rect_stroke(
            r,
            4.0,
            Stroke::new(1.0_f32, Color32::from_rgb(125, 211, 252)),
            egui::StrokeKind::Inside,
        );
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
            painter.rect_stroke(
                br,
                3.0,
                Stroke::new(1.0_f32, Color32::from_rgb(239, 68, 68)),
                egui::StrokeKind::Inside,
            );
            painter.text(badge_p, egui::Align2::CENTER_CENTER, "OL/", FontId::proportional(7.5), Color32::WHITE);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_turntable_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    special_top_mesh: Option<&CachedModelTopMesh2D>,
    gauge: f64,
    rail_w: f32,
    rail_col: Color32,
    ballast_col: Color32,
    ballast_w: f32,
    color_mode: ColorMode,
    is_selected: bool,
    is_active_layer: bool,
    is_editable: bool,
    show_labels: bool,
    power_sim_enabled: bool,
    piece_power: Option<&PiecePowerResult>,
    bridge_angle_deg: f64,
    accessory_buttons: &mut Vec<TrackAccessoryButton>,
) {
    let outer_r = crate::turntables_3d::turntable_outer_radius(piece, entry);
    let scale_f = (outer_r / 189.0).clamp(0.4, 2.5);
    let pit_r = 139.0 * scale_f;
    let ring_rail_r = 124.0 * scale_f;
    let half_b = pit_r - 1.0;
    let hg = gauge * 0.5;
    let center = camera.world_to_screen(rect, piece.transform.x, piece.transform.y);
    let to_scr = |lx: f64, ly: f64| -> Pos2 {
        let rp = rotate(Vec2 { x: lx, y: ly }, piece.transform.rotation);
        camera.world_to_screen(rect, piece.transform.x + rp.x, piece.transform.y + rp.y)
    };
    let to_scr_bridge = |lx: f64, ly: f64| -> Pos2 {
        let rp = rotate(Vec2 { x: lx, y: ly }, piece.transform.rotation + bridge_angle_deg);
        camera.world_to_screen(rect, piece.transform.x + rp.x, piece.transform.y + rp.y)
    };

    // Pass A: Selection halo
    if is_selected {
        painter.circle_filled(
            center,
            (outer_r as f32) * camera.zoom + 4.5,
            Color32::from_rgba_unmultiplied(56, 189, 248, 95),
        );
    }

    // Pass B: Cached orthographic top-down 3D projection of turntable
    if let Some(tm) = special_top_mesh {
        draw_projected_mesh_2d(
            painter,
            rect,
            camera,
            tm,
            piece.transform,
            is_selected,
            is_active_layer,
            piece.has_tunnel == Some(true),
        );
    }

    // When bridge has rotated away from 0°, mask out the static 0° bridge inside the pit so ONLY the live bridge renders!
    let bridge_moved = (bridge_angle_deg % 360.0).abs() > 0.05;
    if bridge_moved {
        painter.circle_filled(
            center,
            ((pit_r - 0.5) as f32) * camera.zoom,
            Color32::from_rgb(51, 65, 85),
        );
        painter.circle_filled(
            center,
            ((24.0 * scale_f) as f32) * camera.zoom,
            Color32::from_rgb(30, 41, 59),
        );
    }

    // Pass C: Crisp zoom-invariant 2D CAD overlays (pit rim, running rail, perimeter status, bridge & stub rails)
    let mut coping_stroke_col = Color32::from_rgb(71, 85, 105);
    let mut running_rail_col = Color32::from_rgba_unmultiplied(186, 198, 212, 210);
    if !is_active_layer && !is_selected {
        coping_stroke_col = desaturate_color(coping_stroke_col, 0.78, 0.72);
        running_rail_col = desaturate_color(running_rail_col, 0.78, 0.72);
    }
    painter.circle_stroke(
        center,
        (pit_r as f32) * camera.zoom,
        Stroke::new((1.5 * camera.zoom).clamp(1.0, 2.5), coping_stroke_col),
    );
    painter.circle_stroke(
        center,
        (ring_rail_r as f32) * camera.zoom,
        Stroke::new((1.25 * camera.zoom).clamp(0.8, 2.2), running_rail_col),
    );

    let (rim_w, rim_col) = if is_selected {
        (2.4_f32, Color32::from_rgb(56, 189, 248))
    } else if power_sim_enabled || color_mode == ColorMode::Layers {
        (if power_sim_enabled { 2.6_f32 } else { 2.2_f32 }, ballast_col)
    } else {
        let col = if is_active_layer {
            Color32::from_rgb(71, 85, 105)
        } else {
            desaturate_color(Color32::from_rgb(71, 85, 105), 0.78, 0.72)
        };
        (1.5_f32, col)
    };
    painter.circle_stroke(center, (outer_r as f32) * camera.zoom, Stroke::new(rim_w, rim_col));

    // Rotating bridge plate girders, operator cabin & twin rails at live `bridge_angle_deg`
    if bridge_moved {
        let gw = (hg + 10.5).max(18.5);
        let deck_poly = vec![
            to_scr_bridge(-half_b, -gw),
            to_scr_bridge(half_b, -gw),
            to_scr_bridge(half_b, gw),
            to_scr_bridge(-half_b, gw),
        ];
        painter.add(egui::Shape::convex_polygon(
            deck_poly,
            Color32::from_rgb(54, 74, 70),
            Stroke::new(1.2_f32, Color32::from_rgb(31, 41, 55)),
        ));
        let cab_cx = -half_b + 36.0 * scale_f;
        let cab_hw = 17.0 * scale_f;
        let cab_poly = vec![
            to_scr_bridge(cab_cx - cab_hw, gw),
            to_scr_bridge(cab_cx + cab_hw, gw),
            to_scr_bridge(cab_cx + cab_hw, gw + 18.0 * scale_f),
            to_scr_bridge(cab_cx - cab_hw, gw + 18.0 * scale_f),
        ];
        painter.add(egui::Shape::convex_polygon(
            cab_poly,
            Color32::from_rgb(63, 76, 89),
            Stroke::new(1.0_f32, Color32::from_rgb(30, 41, 59)),
        ));
    }
    for side in [-1.0_f64, 1.0] {
        painter.line_segment(
            [to_scr_bridge(-half_b, side * hg), to_scr_bridge(half_b, side * hg)],
            Stroke::new(rail_w, rail_col),
        );
    }

    // Radial exit stub twin rails
    let slot_angles: Vec<f64> = entry
        .and_then(|e| e.local_connectors.as_ref())
        .filter(|lc| !lc.is_empty())
        .map(|lc| lc.iter().map(|c| c.angle).collect())
        .unwrap_or_else(|| (0..30).map(|i| i as f64 * 12.0).collect());
    let r_stub_end = outer_r + 0.30;
    for &deg in &slot_angles {
        let rad = deg.to_radians();
        let (ca, sa) = (rad.cos(), rad.sin());
        let (nx, ny) = (-sa, ca);
        for side in [-1.0_f64, 1.0] {
            let ox = nx * side * hg;
            let oy = ny * side * hg;
            painter.line_segment(
                [
                    to_scr(pit_r * ca + ox, pit_r * sa + oy),
                    to_scr(r_stub_end * ca + ox, r_stub_end * sa + oy),
                ],
                Stroke::new(rail_w, rail_col),
            );
        }
    }

    // Pass D: Drehscheiben-Spinne 2D catenary overlay when enabled
    if piece.has_catenary == Some(true) {
        let cat_col = catenary_color_2d(power_sim_enabled, piece_power, is_active_layer, is_selected);
        let mut mast_col = Color32::from_rgb(40, 89, 67);
        if !is_active_layer && !is_selected {
            mast_col = desaturate_color(mast_col, 0.78, 0.72);
        }
        let spider_r = 22.0 * scale_f;
        let outer_holder_r = outer_r - 4.0 * scale_f;
        let wire_w = (1.5 * camera.zoom).clamp(1.0, 2.4);

        for &deg in &slot_angles {
            let rad = deg.to_radians();
            let (ca, sa) = (rad.cos(), rad.sin());
            let (nx, ny) = (-sa, ca);
            painter.line_segment(
                [to_scr(spider_r * ca, spider_r * sa), to_scr(outer_r * ca, outer_r * sa)],
                Stroke::new(wire_w, cat_col),
            );
            if camera.zoom >= 0.14 {
                let hx = outer_holder_r * ca;
                let hy = outer_holder_r * sa;
                let p0 = to_scr(hx - nx * 12.5, hy - ny * 12.5);
                let p1 = to_scr(hx + nx * 12.5, hy + ny * 12.5);
                painter.line_segment([p0, p1], Stroke::new((1.2 * camera.zoom).clamp(0.8, 2.0), mast_col));
                let dot_r = (1.8 * camera.zoom).clamp(1.1, 3.0);
                painter.circle_filled(p0, dot_r, mast_col);
                painter.circle_filled(p1, dot_r, mast_col);
            }
        }
        painter.line_segment([to_scr(-spider_r, 0.0), to_scr(spider_r, 0.0)], Stroke::new(wire_w, cat_col));
        painter.circle_stroke(
            center,
            (spider_r as f32) * camera.zoom,
            Stroke::new((2.2 * camera.zoom).clamp(1.2, 3.4), mast_col),
        );
        painter.circle_stroke(center, (spider_r as f32) * camera.zoom, Stroke::new(wire_w, cat_col));
    }

    // Pass E: Centered article-number label & badges in open pit (opposite Wärterhaus cabin)
    let label_rp = rotate(Vec2 { x: 0.0, y: -pit_r * 0.48 }, piece.transform.rotation);
    let label_world = Vec2 {
        x: piece.transform.x + label_rp.x,
        y: piece.transform.y + label_rp.y,
    };
    draw_piece_label_and_badges(
        painter,
        rect,
        camera,
        piece,
        entry,
        label_world,
        ballast_w,
        gauge,
        false,
        is_selected,
        is_active_layer,
        is_editable,
        show_labels,
        power_sim_enabled,
        piece_power,
        None,
        accessory_buttons,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_transfer_table_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    piece: &TrackPiece,
    all_pieces: &[TrackPiece],
    piece_id_to_idx: &HashMap<String, usize>,
    entry: Option<&CatalogEntry>,
    special_top_mesh: Option<&CachedModelTopMesh2D>,
    gauge: f64,
    rail_w: f32,
    rail_col: Color32,
    ballast_col: Color32,
    ballast_w: f32,
    color_mode: ColorMode,
    is_selected: bool,
    is_active_layer: bool,
    is_editable: bool,
    show_labels: bool,
    power_sim_enabled: bool,
    piece_power: Option<&PiecePowerResult>,
    bridge_z_override: Option<f64>,
    accessory_buttons: &mut Vec<TrackAccessoryButton>,
) {
    let (_, l, top_z, bot_z) = crate::turntables_3d::transfer_table_local_bounds(entry);
    let pit_left = 36.0;
    let pit_right = l - 36.0;
    let pit_top = top_z + 6.0;
    let pit_bot = bot_z - 6.0;
    let track_zs = crate::turntables_3d::extract_transfer_track_zs(entry);
    let bridge_cx = l * 0.5;
    let default_bridge_z = track_zs[track_zs.len() / 2] as f64;
    let bridge_z = bridge_z_override.unwrap_or(default_bridge_z);
    let bridge_shifted = (bridge_z - default_bridge_z).abs() > 0.2;
    let hg = gauge * 0.5;
    let girder_z_off = (hg + 4.2).max(12.6);
    let rail_z_off = girder_z_off + 7.7;

    let to_scr = |lx: f64, ly: f64| -> Pos2 {
        let rp = rotate(Vec2 { x: lx, y: ly }, piece.transform.rotation);
        camera.world_to_screen(rect, piece.transform.x + rp.x, piece.transform.y + rp.y)
    };
    let quad_pts = |x0: f64, x1: f64, y0: f64, y1: f64| -> Vec<Pos2> {
        vec![to_scr(x0, y0), to_scr(x1, y0), to_scr(x1, y1), to_scr(x0, y1)]
    };

    // Pass A: Selection halo
    if is_selected {
        let pad = 4.5 / (camera.zoom as f64).max(0.1);
        painter.add(egui::Shape::convex_polygon(
            quad_pts(-pad, l + pad, top_z - pad, bot_z + pad),
            Color32::from_rgba_unmultiplied(56, 189, 248, 95),
            Stroke::NONE,
        ));
    }

    // Pass B: Cached orthographic top-down 3D projection of transfer table
    if let Some(tm) = special_top_mesh {
        draw_projected_mesh_2d(
            painter,
            rect,
            camera,
            tm,
            piece.transform,
            is_selected,
            is_active_layer,
            piece.has_tunnel == Some(true),
        );
    }

    // Pass C: Crisp zoom-invariant 2D CAD overlays
    if bridge_shifted {
        painter.add(egui::Shape::convex_polygon(
            quad_pts(pit_left + 0.5, pit_right - 0.5, pit_top + 0.5, pit_bot - 0.5),
            Color32::from_rgb(51, 65, 85),
            Stroke::NONE,
        ));
    }
    let mut coping_stroke_col = Color32::from_rgb(71, 85, 105);
    let mut running_rail_col = Color32::from_rgba_unmultiplied(186, 198, 212, 210);
    if !is_active_layer && !is_selected {
        coping_stroke_col = desaturate_color(coping_stroke_col, 0.78, 0.72);
        running_rail_col = desaturate_color(running_rail_col, 0.78, 0.72);
    }
    painter.add(egui::Shape::closed_line(
        quad_pts(pit_left, pit_right, pit_top, pit_bot),
        Stroke::new((1.5 * camera.zoom).clamp(1.0, 2.4), coping_stroke_col),
    ));

    for rx in [pit_left + 44.0, l * 0.5, pit_right - 44.0] {
        painter.line_segment(
            [to_scr(rx, pit_top + 6.0), to_scr(rx, pit_bot - 6.0)],
            Stroke::new((1.25 * camera.zoom).clamp(0.8, 2.2), running_rail_col),
        );
    }

    // Twin rails on all approach stubs & movable bridge
    for &tzf in &track_zs {
        let tz = tzf as f64;
        for side in [-1.0_f64, 1.0] {
            let rz = tz + side * hg;
            painter.line_segment([to_scr(-0.30, rz), to_scr(pit_left, rz)], Stroke::new(rail_w, rail_col));
            painter.line_segment([to_scr(pit_right, rz), to_scr(l + 0.30, rz)], Stroke::new(rail_w, rail_col));
        }
    }
    if bridge_shifted {
        let gw = (hg + 10.5).max(18.5);
        painter.add(egui::Shape::convex_polygon(
            quad_pts(pit_left + 1.0, pit_right - 1.0, bridge_z - gw, bridge_z + gw),
            Color32::from_rgb(54, 74, 70),
            Stroke::new(1.2_f32, Color32::from_rgb(31, 41, 55)),
        ));
    }
    for side in [-1.0_f64, 1.0] {
        let rz = bridge_z + side * hg;
        painter.line_segment(
            [to_scr(pit_left + 1.0, rz), to_scr(pit_right - 1.0, rz)],
            Stroke::new(rail_w, rail_col),
        );
    }

    // Overhead power-collector portal arch top-pass above bridge rails
    let mut arch_col = Color32::from_rgb(54, 74, 70);
    let mut arch_trim = Color32::from_rgb(31, 41, 55);
    let mut ins_col = Color32::from_rgb(124, 45, 18);
    let mut bronze_col = Color32::from_rgb(180, 83, 9);
    if !is_active_layer && !is_selected {
        arch_col = desaturate_color(arch_col, 0.78, 0.72);
        arch_trim = desaturate_color(arch_trim, 0.78, 0.72);
        ins_col = desaturate_color(ins_col, 0.78, 0.72);
        bronze_col = desaturate_color(bronze_col, 0.78, 0.72);
    }
    painter.add(egui::Shape::convex_polygon(
        quad_pts(
            bridge_cx - 1.9,
            bridge_cx + 1.9,
            bridge_z - (rail_z_off + 2.0),
            bridge_z + (rail_z_off + 2.0),
        ),
        arch_col,
        Stroke::new((1.0 * camera.zoom).clamp(0.7, 1.6), arch_trim),
    ));
    let ins_r = (1.6 * camera.zoom).clamp(1.1, 3.0);
    for sz in [-1.0_f64, 1.0] {
        painter.circle_filled(to_scr(bridge_cx, bridge_z + sz * 8.0), ins_r, ins_col);
    }
    painter.line_segment(
        [to_scr(bridge_cx, bridge_z - 7.0), to_scr(bridge_cx, bridge_z + 7.0)],
        Stroke::new((1.8 * camera.zoom).clamp(1.2, 3.2), bronze_col),
    );

    let (rim_w, rim_col) = if is_selected {
        (2.4_f32, Color32::from_rgb(56, 189, 248))
    } else if power_sim_enabled || color_mode == ColorMode::Layers {
        (if power_sim_enabled { 2.6_f32 } else { 2.2_f32 }, ballast_col)
    } else {
        let col = if is_active_layer {
            Color32::from_rgb(71, 85, 105)
        } else {
            desaturate_color(Color32::from_rgb(71, 85, 105), 0.78, 0.72)
        };
        (1.5_f32, col)
    };
    painter.add(egui::Shape::closed_line(
        quad_pts(0.0, l, top_z, bot_z),
        Stroke::new(rim_w, rim_col),
    ));

    // Pass D: Bridge catenary + connected catenary approach stubs (matching turntables_3d)
    if piece.has_catenary == Some(true) {
        let cat_col = catenary_color_2d(power_sim_enabled, piece_power, is_active_layer, is_selected);
        let mut mast_col = Color32::from_rgb(40, 89, 67);
        if !is_active_layer && !is_selected {
            mast_col = desaturate_color(mast_col, 0.78, 0.72);
        }
        let wire_w = (1.6 * camera.zoom).clamp(1.0, 2.5);
        let portal_w = (1.2 * camera.zoom).clamp(0.8, 2.0);
        let dot_r = (1.9 * camera.zoom).clamp(1.2, 3.2);

        painter.line_segment(
            [to_scr(pit_left + 1.0, bridge_z), to_scr(pit_right - 1.0, bridge_z)],
            Stroke::new(wire_w, cat_col),
        );
        for px in [pit_left + 12.0, pit_right - 12.0] {
            let p0 = to_scr(px, bridge_z - 15.5);
            let p1 = to_scr(px, bridge_z + 15.5);
            painter.line_segment([p0, p1], Stroke::new(portal_w, mast_col));
            painter.circle_filled(p0, dot_r, mast_col);
            painter.circle_filled(p1, dot_r, mast_col);
        }

        let local_conns = entry.and_then(|e| e.local_connectors.as_ref());
        for (idx, conn) in piece.connectors.iter().enumerate() {
            let Some(ref cref) = conn.connected_to else {
                continue;
            };
            let other_cat = piece_id_to_idx
                .get(&cref.piece_id)
                .and_then(|&pi| all_pieces.get(pi))
                .map_or(false, |op| op.has_catenary == Some(true));
            if !other_cat {
                continue;
            }
            let lc = local_conns.and_then(|v| v.get(idx));
            let is_left = lc.map_or(idx < track_zs.len(), |c| c.x < bridge_cx);
            let tz = lc.map_or_else(|| track_zs[idx % track_zs.len()] as f64, |c| c.y);
            let (wx0, wx1, px, edge_x) = if is_left {
                (0.0, pit_left, pit_left - 2.5, 0.0)
            } else {
                (pit_right, l, pit_right + 2.5, l)
            };
            painter.line_segment([to_scr(wx0, tz), to_scr(wx1, tz)], Stroke::new(wire_w, cat_col));
            let p0 = to_scr(px, tz - 14.5);
            let p1 = to_scr(px, tz + 14.5);
            painter.line_segment([p0, p1], Stroke::new(portal_w, mast_col));
            painter.circle_filled(p0, dot_r, mast_col);
            painter.circle_filled(p1, dot_r, mast_col);
            let sm = to_scr(edge_x, tz + 24.0);
            let sc = to_scr(edge_x, tz);
            painter.line_segment([sm, sc], Stroke::new((1.4 * camera.zoom).clamp(1.0, 2.2), mast_col));
            painter.circle_filled(sm, dot_r * 1.1, mast_col);
        }
    }

    // Pass E: Article-number label & badges in open pit above bridge
    let label_rp = rotate(Vec2 { x: bridge_cx, y: bridge_z - 56.0 }, piece.transform.rotation);
    let label_world = Vec2 {
        x: piece.transform.x + label_rp.x,
        y: piece.transform.y + label_rp.y,
    };
    draw_piece_label_and_badges(
        painter,
        rect,
        camera,
        piece,
        entry,
        label_world,
        ballast_w,
        gauge,
        false,
        is_selected,
        is_active_layer,
        is_editable,
        show_labels,
        power_sim_enabled,
        piece_power,
        None,
        accessory_buttons,
    );
}

