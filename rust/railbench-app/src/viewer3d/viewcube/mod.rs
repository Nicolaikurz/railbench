mod geometry;

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke,
};
use railbench_core::types::Layout;

use super::{dot3, normalize3, Camera3D, CameraViewPreset};
use geometry::*;

fn paint_curved_arrow_arc(
    painter: &egui::Painter,
    center: Pos2,
    r_in: f32,
    r_out: f32,
    start_deg: f32,
    end_deg: f32,
    fill: Color32,
    stroke: Stroke,
) {
    let steps = 16usize;
    let tip_sweep = (end_deg - start_deg).signum() * 10.5;
    let body_end_deg = end_deg - tip_sweep;
    let pt_polar = |r: f32, deg: f32| -> Pos2 {
        let rad = deg.to_radians();
        Pos2::new(center.x + r * rad.cos(), center.y + r * rad.sin())
    };

    let mut outer_pts = Vec::with_capacity(steps + 1);
    let mut inner_pts = Vec::with_capacity(steps + 1);
    let mut mesh = egui::Mesh::default();
    for s in 0..=steps {
        let u = s as f32 / steps as f32;
        let ang = start_deg + (body_end_deg - start_deg) * u;
        let po = pt_polar(r_out, ang);
        let pi = pt_polar(r_in, ang);
        outer_pts.push(po);
        inner_pts.push(pi);
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(po, fill);
        mesh.colored_vertex(pi, fill);
        if s > 0 {
            mesh.add_triangle(base - 2, base, base + 1);
            mesh.add_triangle(base - 2, base + 1, base - 1);
        }
    }

    let r_mid = (r_in + r_out) * 0.5;
    let tip = pt_polar(r_mid, end_deg);
    let base_out = pt_polar(r_out + 2.6, body_end_deg);
    let base_in = pt_polar(r_in - 2.6, body_end_deg);
    let head_base = mesh.vertices.len() as u32;
    mesh.colored_vertex(tip, fill);
    mesh.colored_vertex(base_in, fill);
    mesh.colored_vertex(base_out, fill);
    mesh.add_triangle(head_base, head_base + 1, head_base + 2);
    painter.add(egui::Shape::mesh(mesh));

    // Continuous outer/arrowhead/inner perimeter line
    let mut border_pts = Vec::with_capacity(steps * 2 + 5);
    border_pts.extend_from_slice(&outer_pts);
    border_pts.push(base_out);
    border_pts.push(tip);
    border_pts.push(base_in);
    for p in inner_pts.iter().rev() {
        border_pts.push(*p);
    }
    painter.add(egui::Shape::closed_line(border_pts, stroke));
}

fn paint_home_badge(
    painter: &egui::Painter,
    center: Pos2,
    fill: Color32,
    border: Stroke,
    icon_col: Color32,
) {
    painter.circle_filled(center, 9.5, fill);
    painter.circle_stroke(center, 9.5, border);

    // Pitched vector roof chevron + walls body
    let roof = vec![
        Pos2::new(center.x - 5.0, center.y - 0.5),
        Pos2::new(center.x, center.y - 4.8),
        Pos2::new(center.x + 5.0, center.y - 0.5),
    ];
    painter.add(egui::Shape::line(roof, Stroke::new(1.5_f32, icon_col)));
    let body = Rect::from_min_max(
        Pos2::new(center.x - 3.3, center.y - 0.3),
        Pos2::new(center.x + 3.3, center.y + 4.2),
    );
    painter.rect_stroke(body, 0.8, Stroke::new(1.3_f32, icon_col), egui::StrokeKind::Inside);
}

fn paint_mini_cube_btn(
    painter: &egui::Painter,
    center: Pos2,
    top_fill: Color32,
    left_fill: Color32,
    right_fill: Color32,
    edge_stroke: Stroke,
) {
    let r_mc = 8.4_f32;
    let dx = r_mc * 0.866;
    let dy = r_mc * 0.5;
    let top = Pos2::new(center.x, center.y - r_mc);
    let tr = Pos2::new(center.x + dx, center.y - dy);
    let br = Pos2::new(center.x + dx, center.y + dy);
    let bot = Pos2::new(center.x, center.y + r_mc);
    let bl = Pos2::new(center.x - dx, center.y + dy);
    let tl = Pos2::new(center.x - dx, center.y - dy);

    // Pass 1: shaded faces with thin self-colored stroke to seal subpixel seams
    painter.add(egui::Shape::convex_polygon(
        vec![top, tr, center, tl],
        top_fill,
        Stroke::new(0.6_f32, top_fill),
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![tl, center, bot, bl],
        left_fill,
        Stroke::new(0.6_f32, left_fill),
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![center, tr, br, bot],
        right_fill,
        Stroke::new(0.6_f32, right_fill),
    ));

    // Pass 2: clean deduplicated internal Y-edges and single closed outer hexagon perimeter
    painter.line_segment([center, tl], edge_stroke);
    painter.line_segment([center, tr], edge_stroke);
    painter.line_segment([center, bot], edge_stroke);
    painter.add(egui::Shape::closed_line(
        vec![top, tr, br, bot, bl, tl],
        edge_stroke,
    ));
}

pub(super) fn draw_viewcube_widget(
    ui: &mut egui::Ui,
    viewport_rect: Rect,
    top_y: f32,
    camera: &mut Camera3D,
    layout: &Layout,
    is_de: bool,
    dark_mode: bool,
) {
    let p = crate::theme::palette(dark_mode);
    const STAGE_SIZE: f32 = 152.0;

    let menu_id = egui::Id::new("hud_3d_viewcube_preset_menu_open");
    let mut menu_open = ui.ctx().data(|d| d.get_temp::<bool>(menu_id)).unwrap_or(false);

    let area_pos = Pos2::new(viewport_rect.min.x + 12.0, top_y);
    let area_resp = egui::Area::new(egui::Id::new("hud_3d_viewcube"))
        .fixed_pos(area_pos)
        .show(ui.ctx(), |ui| {
            let (stage_rect, stage_resp) =
                ui.allocate_exact_size(egui::vec2(STAGE_SIZE, STAGE_SIZE), Sense::click_and_drag());

            if stage_resp.dragged_by(egui::PointerButton::Primary) {
                let d = stage_resp.drag_delta();
                camera.yaw -= d.x * 0.015;
                camera.pitch = (camera.pitch + d.y * 0.015).clamp(-1.48, 1.48);
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }

            let painter = ui.painter_at(stage_rect);
            let center = stage_rect.center();
            let hover_pos = ui.input(|i| i.pointer.hover_pos());

            // Theme-harmonized opaque palette (warm crema in light mode, slate navy in dark mode)
            let (
                main_shadow,
                main_lit,
                bevel_shadow,
                bevel_lit,
                crease_col,
                silhouette_col,
                ctrl_fill,
                ctrl_border,
                ctrl_icon,
                hover_fill,
                hover_border,
                text_idle,
                mc_top_idle,
                mc_left_idle,
                mc_right_idle,
                triad_backing,
            ) = if dark_mode {
                (
                    [28.0_f32, 39.0, 58.0],
                    [58.0_f32, 76.0, 106.0],
                    [21.0_f32, 30.0, 46.0],
                    [45.0_f32, 60.0, 84.0],
                    Color32::from_rgba_unmultiplied(148, 163, 184, 95),
                    Color32::from_rgba_unmultiplied(10, 15, 28, 235),
                    Color32::from_rgba_unmultiplied(p.bg_card.r(), p.bg_card.g(), p.bg_card.b(), 238),
                    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 110)),
                    p.text_sub,
                    p.primary_hover,
                    Stroke::new(1.4_f32, p.primary),
                    Color32::from_rgb(226, 232, 240),
                    Color32::from_rgb(56, 74, 102),
                    Color32::from_rgb(38, 52, 74),
                    Color32::from_rgb(26, 37, 55),
                    Color32::from_rgba_unmultiplied(8, 13, 24, 185),
                )
            } else {
                (
                    [220.0_f32, 211.0, 196.0],
                    [252.0_f32, 249.0, 242.0],
                    [204.0_f32, 194.0, 177.0],
                    [238.0_f32, 231.0, 217.0],
                    Color32::from_rgba_unmultiplied(150, 138, 120, 135),
                    Color32::from_rgba_unmultiplied(92, 82, 68, 215),
                    Color32::from_rgba_unmultiplied(p.bg_card.r(), p.bg_card.g(), p.bg_card.b(), 244),
                    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(172, 162, 146, 215)),
                    p.text_sub,
                    p.primary,
                    Stroke::new(1.4_f32, p.primary_hover),
                    Color32::from_rgb(52, 46, 38),
                    Color32::from_rgb(252, 249, 242),
                    Color32::from_rgb(236, 229, 214),
                    Color32::from_rgb(218, 208, 192),
                    Color32::from_rgba_unmultiplied(255, 252, 246, 185),
                )
            };

            let (right, up, view_z) = camera_basis(camera);
            let mut any_interactive_hov = false;
            let mut hovered_tip: Option<&str> = None;

            // Soft translucent drop shadow beneath floating cube (no container box)
            painter.circle_filled(
                center + egui::vec2(0.0, 4.0),
                36.0,
                Color32::from_black_alpha(if dark_mode { 20 } else { 12 }),
            );
            painter.circle_filled(
                center + egui::vec2(0.0, 2.0),
                27.0,
                Color32::from_black_alpha(if dark_mode { 10 } else { 6 }),
            );

            // 1. Directional Triangle Orbit Arrows (sleek 11x7.5 px proportions)
            let dir_arrows: [(Pos2, [Pos2; 3], &str, f32, f32); 4] = [
                (
                    Pos2::new(center.x - 54.0, center.y),
                    [
                        Pos2::new(center.x - 58.0, center.y),
                        Pos2::new(center.x - 50.5, center.y - 5.5),
                        Pos2::new(center.x - 50.5, center.y + 5.5),
                    ],
                    if is_de { "45° links drehen" } else { "Orbit 45 deg left" },
                    -std::f32::consts::FRAC_PI_4,
                    0.0,
                ),
                (
                    Pos2::new(center.x + 54.0, center.y),
                    [
                        Pos2::new(center.x + 58.0, center.y),
                        Pos2::new(center.x + 50.5, center.y - 5.5),
                        Pos2::new(center.x + 50.5, center.y + 5.5),
                    ],
                    if is_de { "45° rechts drehen" } else { "Orbit 45 deg right" },
                    std::f32::consts::FRAC_PI_4,
                    0.0,
                ),
                (
                    Pos2::new(center.x, center.y - 54.0),
                    [
                        Pos2::new(center.x, center.y - 58.0),
                        Pos2::new(center.x - 5.5, center.y - 50.5),
                        Pos2::new(center.x + 5.5, center.y - 50.5),
                    ],
                    if is_de { "Nach oben kippen" } else { "Tilt up 30 deg" },
                    0.0,
                    0.52,
                ),
                (
                    Pos2::new(center.x, center.y + 54.0),
                    [
                        Pos2::new(center.x, center.y + 58.0),
                        Pos2::new(center.x - 5.5, center.y + 50.5),
                        Pos2::new(center.x + 5.5, center.y + 50.5),
                    ],
                    if is_de { "Nach unten kippen" } else { "Tilt down 30 deg" },
                    0.0,
                    -0.52,
                ),
            ];
            let mut arrow_delta: Option<(f32, f32)> = None;
            for (hit_c, tri, tip, dyaw, dpitch) in dir_arrows {
                let hit_r = Rect::from_center_size(hit_c, egui::vec2(17.0, 17.0));
                let hov = hover_pos.is_some_and(|hp| hit_r.contains(hp));
                any_interactive_hov |= hov;
                if hov {
                    hovered_tip = Some(tip);
                    if stage_resp.clicked() {
                        arrow_delta = Some((dyaw, dpitch));
                    }
                }
                painter.add(egui::Shape::convex_polygon(
                    tri.to_vec(),
                    if hov { hover_fill } else { ctrl_fill },
                    if hov { hover_border } else { ctrl_border },
                ));
            }

            // 2. Two Seamless Welded Top Curved Rotation Arrows (-90 deg CCW / +90 deg CW)
            let arc_defs: [(f32, f32, f32, &str); 2] = [
                (-109.0, -148.0, -std::f32::consts::FRAC_PI_2, if is_de { "90° links drehen" } else { "Rotate 90 deg CCW" }),
                (-71.0, -32.0, std::f32::consts::FRAC_PI_2, if is_de { "90° rechts drehen" } else { "Rotate 90 deg CW" }),
            ];
            let mut roll_delta: Option<f32> = None;
            for (a0_deg, a1_deg, dyaw, tip) in arc_defs {
                let mid_rad = ((a0_deg + a1_deg) * 0.5).to_radians();
                let hit_pos = Pos2::new(center.x + 47.2 * mid_rad.cos(), center.y + 47.2 * mid_rad.sin());
                let hov = hover_pos.is_some_and(|hp| hp.distance(hit_pos) <= 13.0);
                any_interactive_hov |= hov;
                if hov {
                    hovered_tip = Some(tip);
                    if stage_resp.clicked() {
                        roll_delta = Some(dyaw);
                    }
                }
                paint_curved_arrow_arc(
                    &painter,
                    center,
                    44.5,
                    50.0,
                    a0_deg,
                    a1_deg,
                    if hov { hover_fill } else { ctrl_fill },
                    if hov { hover_border } else { ctrl_border },
                );
            }

            // 3. Top-Right Floating Vector Home Badge
            let home_center = Pos2::new(stage_rect.max.x - 17.0, stage_rect.min.y + 17.0);
            let home_hov = hover_pos.is_some_and(|hp| hp.distance(home_center) <= 11.5);
            any_interactive_hov |= home_hov;
            let mut go_home = false;
            if home_hov {
                hovered_tip = Some(if is_de {
                    "Standard 3D-Ansicht (ISO)"
                } else {
                    "Home 3D Isometric View"
                });
                if stage_resp.clicked() {
                    go_home = true;
                }
            }
            paint_home_badge(
                &painter,
                home_center,
                if home_hov { hover_fill } else { ctrl_fill },
                if home_hov { hover_border } else { ctrl_border },
                if home_hov { Color32::WHITE } else { ctrl_icon },
            );

            // 4. Bottom-Right Mini Isometric Cube Preset Menu Button
            let mini_center = Pos2::new(stage_rect.max.x - 18.0, stage_rect.max.y - 18.0);
            let mini_hov = hover_pos.is_some_and(|hp| hp.distance(mini_center) <= 12.0);
            any_interactive_hov |= mini_hov;
            if mini_hov {
                hovered_tip = Some(if is_de { "Ansicht-Presets" } else { "Camera View Presets" });
                if stage_resp.clicked() {
                    menu_open = !menu_open;
                }
            }
            let mc_active = mini_hov || menu_open;
            let (mc_t, mc_l, mc_r, mc_stroke) = if mc_active {
                let hover_dim = lerp_rgb_opaque(
                    [hover_fill.r() as f32, hover_fill.g() as f32, hover_fill.b() as f32],
                    [10.0, 25.0, 48.0],
                    0.22,
                );
                let hover_dark = lerp_rgb_opaque(
                    [hover_fill.r() as f32, hover_fill.g() as f32, hover_fill.b() as f32],
                    [10.0, 25.0, 48.0],
                    0.38,
                );
                (hover_fill, hover_dim, hover_dark, hover_border)
            } else {
                (mc_top_idle, mc_left_idle, mc_right_idle, ctrl_border)
            };
            paint_mini_cube_btn(&painter, mini_center, mc_t, mc_l, mc_r, mc_stroke);

            // 5. Project, Cull & 2-Pass Render the 26-Facet Chamfered Cube
            let visible_facets = collect_visible_facets(
                build_chamfered_cube_facets(is_de),
                center,
                camera,
            );
            let hovered_facet_idx = if !any_interactive_hov {
                hover_pos.and_then(|hp| {
                    visible_facets
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, f)| point_in_polygon_2d(hp, &f.screen_poly))
                        .map(|(idx, _)| idx)
                })
            } else {
                None
            };
            any_interactive_hov |= hovered_facet_idx.is_some();

            let light_dir = normalize3([0.38, 0.84, 0.38]);
            // Pass 1: Opaque facet fills with thin self-colored stroke to eliminate AA cracks
            let mut clicked_normal: Option<[f32; 3]> = None;
            for (idx, f) in visible_facets.iter().enumerate() {
                let is_hov = hovered_facet_idx == Some(idx);
                let fill = if is_hov {
                    hover_fill
                } else {
                    let ldot = (0.5 + 0.5 * dot3(f.normal, light_dir)).clamp(0.0, 1.0);
                    if f.label.is_some() {
                        lerp_rgb_opaque(main_shadow, main_lit, ldot)
                    } else {
                        lerp_rgb_opaque(bevel_shadow, bevel_lit, ldot)
                    }
                };
                painter.add(egui::Shape::convex_polygon(
                    f.screen_poly.clone(),
                    fill,
                    Stroke::new(0.75_f32, fill),
                ));
                if is_hov && stage_resp.clicked() {
                    clicked_normal = Some(f.normal);
                }
            }

            // Pass 2: Deduplicated internal creases (count >= 2) and crisp outer silhouette (count == 1)
            let dedup_edges = dedup_visible_facet_edges(&visible_facets);
            let crease_stroke = Stroke::new(0.85_f32, crease_col);
            let silhouette_stroke = Stroke::new(1.35_f32, silhouette_col);
            for (p0, p1, count) in &dedup_edges {
                if *count >= 2 {
                    painter.line_segment([*p0, *p1], crease_stroke);
                }
            }
            for (p0, p1, count) in &dedup_edges {
                if *count == 1 {
                    painter.line_segment([*p0, *p1], silhouette_stroke);
                }
            }
            if let Some(h_idx) = hovered_facet_idx {
                if let Some(hf) = visible_facets.get(h_idx) {
                    painter.add(egui::Shape::closed_line(hf.screen_poly.clone(), hover_border));
                }
            }

            // Pass 3: Main face labels guarded by nz >= 0.40 & minimum projected bounding box
            for (idx, f) in visible_facets.iter().enumerate() {
                let Some(lbl) = f.label else { continue };
                if f.nz < 0.40 || f.screen_poly.is_empty() {
                    continue;
                }
                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;
                let mut sx = 0.0_f32;
                let mut sy = 0.0_f32;
                for pt in &f.screen_poly {
                    min_x = min_x.min(pt.x);
                    max_x = max_x.max(pt.x);
                    min_y = min_y.min(pt.y);
                    max_y = max_y.max(pt.y);
                    sx += pt.x;
                    sy += pt.y;
                }
                if (max_x - min_x) < 26.0 || (max_y - min_y) < 14.0 {
                    continue;
                }
                let inv = 1.0 / f.screen_poly.len() as f32;
                let centroid = Pos2::new(sx * inv, sy * inv);
                let is_hov = hovered_facet_idx == Some(idx);
                let fade = ((f.nz - 0.40) / 0.16).clamp(0.0, 1.0);
                let txt_col = if is_hov {
                    Color32::WHITE
                } else {
                    Color32::from_rgba_unmultiplied(
                        text_idle.r(),
                        text_idle.g(),
                        text_idle.b(),
                        (fade * 255.0).round() as u8,
                    )
                };
                painter.text(
                    centroid,
                    egui::Align2::CENTER_CENTER,
                    lbl,
                    FontId::proportional(9.5),
                    txt_col,
                );
            }

            // 6. Bottom-Left Live 3D XYZ RGB Coordinate Triad (padded well inside stage_rect)
            let triad_origin = Pos2::new(stage_rect.min.x + 24.0, stage_rect.max.y - 24.0);
            let axis_len = 16.5_f32;
            let label_bounds = stage_rect.shrink(6.0);
            let mut triad_axes: [(f32, [f32; 3], Color32, &str); 3] = [
                (dot3([1.0, 0.0, 0.0], view_z), [1.0, 0.0, 0.0], Color32::from_rgb(239, 68, 68), "X"),
                (dot3([0.0, 1.0, 0.0], view_z), [0.0, 1.0, 0.0], Color32::from_rgb(34, 197, 94), "Y"),
                (dot3([0.0, 0.0, 1.0], view_z), [0.0, 0.0, 1.0], Color32::from_rgb(59, 130, 246), "Z"),
            ];
            triad_axes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            for (_, ax, col, axis_lbl) in triad_axes {
                let dx = dot3(ax, right) * axis_len;
                let dy = -dot3(ax, up) * axis_len;
                let end = Pos2::new(triad_origin.x + dx, triad_origin.y + dy);
                painter.line_segment([triad_origin, end], Stroke::new(2.6_f32, triad_backing));
                painter.line_segment([triad_origin, end], Stroke::new(1.8_f32, col));
                painter.circle_filled(end, 1.9, col);
                let len2 = dx.hypot(dy);
                if len2 > 7.0 {
                    let raw_lbl_pos = Pos2::new(
                        end.x + (dx / len2) * 5.5,
                        end.y + (dy / len2) * 5.5,
                    );
                    let lbl_pos = label_bounds.clamp(raw_lbl_pos);
                    painter.text(
                        lbl_pos,
                        egui::Align2::CENTER_CENTER,
                        axis_lbl,
                        FontId::proportional(8.5),
                        col,
                    );
                }
            }
            painter.circle_filled(
                triad_origin,
                2.0,
                if dark_mode {
                    Color32::from_rgb(226, 232, 240)
                } else {
                    Color32::from_rgb(76, 68, 58)
                },
            );

            if any_interactive_hov && !stage_resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if let Some(tip) = hovered_tip {
                stage_resp.clone().on_hover_text(tip);
            }

            // Apply ViewCube clicks cleanly after rendering
            if go_home {
                camera.apply_preset(CameraViewPreset::Home, layout);
            } else if let Some((dy, dp)) = arrow_delta {
                camera.yaw += dy;
                camera.pitch = (camera.pitch + dp).clamp(-1.48, 1.48);
            } else if let Some(dy) = roll_delta {
                camera.yaw += dy;
            } else if let Some(n) = clicked_normal {
                apply_facet_normal_to_camera(camera, n);
            }
        });

    // Floating preset menu triggered by the bottom-right mini-cube icon
    if menu_open {
        let menu_pos = Pos2::new(
            area_resp.response.rect.max.x - 18.0,
            area_resp.response.rect.max.y + 4.0,
        );
        let pop = egui::Area::new(egui::Id::new("hud_3d_viewcube_preset_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(menu_pos)
            .constrain_to(viewport_rect.shrink(6.0))
            .show(ui.ctx(), |ui| {
                super::viewport::hud_3d_panel_frame(dark_mode, None).show(ui, |ui| {
                    ui.set_min_width(116.0);
                    ui.spacing_mut().item_spacing.y = 3.0;
                    let presets: [(&str, Option<CameraViewPreset>); 8] = [
                        (if is_de { "Standard (ISO)" } else { "Home (ISO)" }, Some(CameraViewPreset::Home)),
                        (if is_de { "Alles einpassen" } else { "Fit View" }, None),
                        (if is_de { "Oben" } else { "Top" }, Some(CameraViewPreset::Top)),
                        (if is_de { "Vorne" } else { "Front" }, Some(CameraViewPreset::Front)),
                        (if is_de { "Rechts" } else { "Right" }, Some(CameraViewPreset::Right)),
                        (if is_de { "Links" } else { "Left" }, Some(CameraViewPreset::Left)),
                        (if is_de { "Hinten" } else { "Back" }, Some(CameraViewPreset::Back)),
                        (if is_de { "Unten" } else { "Bottom" }, Some(CameraViewPreset::Bottom)),
                    ];
                    for (lbl, preset) in presets {
                        if ui
                            .add_sized(
                                [ui.available_width(), 22.0],
                                egui::Button::new(RichText::new(lbl).size(11.0).color(p.text_main))
                                    .fill(p.bg_input)
                                    .stroke(Stroke::new(1.0_f32, p.border_main))
                                    .corner_radius(CornerRadius::same(5)),
                            )
                            .clicked()
                        {
                            match preset {
                                Some(pr) => camera.apply_preset(pr, layout),
                                None => camera.fit_layout(layout),
                            }
                            menu_open = false;
                        }
                    }
                });
            });

        if ui.input(|i| i.pointer.primary_pressed()) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                if !pop.response.rect.contains(pos) && !area_resp.response.rect.contains(pos) {
                    menu_open = false;
                }
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            menu_open = false;
        }
    }

    ui.ctx().data_mut(|d| d.insert_temp(menu_id, menu_open));
}

