mod geometry;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke};
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

    let area_pos = Pos2::new(viewport_rect.min.x + 12.0, top_y);
    egui::Area::new(egui::Id::new("hud_3d_viewcube"))
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
                )
            };

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

            // 4. Project, Cull & 2-Pass Mesh Render the 26-Facet Chamfered Cube
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
                        .find(|(_, f)| f.screen_area >= 12.0 && point_in_polygon_2d(hp, &f.screen_poly))
                        .map(|(idx, _)| idx)
                })
            } else {
                None
            };
            any_interactive_hov |= hovered_facet_idx.is_some();

            let light_dir = normalize3([0.38, 0.84, 0.38]);
            // Pass 1: Watertight triangle-fan Mesh fill (zero AA seams & zero acute miter spikes)
            let mut clicked_normal: Option<[f32; 3]> = None;
            let mut cube_mesh = egui::Mesh::default();
            for (idx, f) in visible_facets.iter().enumerate() {
                if f.screen_poly.len() < 3 {
                    continue;
                }
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
                let base = cube_mesh.vertices.len() as u32;
                for &pt in &f.screen_poly {
                    cube_mesh.colored_vertex(pt, fill);
                }
                for i in 1..(f.screen_poly.len() as u32 - 1) {
                    cube_mesh.add_triangle(base, base + i, base + i + 1);
                }
                if is_hov && stage_resp.clicked() {
                    clicked_normal = Some(f.normal);
                }
            }
            painter.add(egui::Shape::mesh(cube_mesh));

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
                    let n = hf.screen_poly.len();
                    for i in 0..n {
                        painter.line_segment(
                            [hf.screen_poly[i], hf.screen_poly[(i + 1) % n]],
                            hover_border,
                        );
                    }
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
}

