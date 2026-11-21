use super::*;

pub(super) fn build_feathered_ribbon_mesh_2d(
    rect: Rect,
    camera: &Camera2D,
    pts: &[Vec2],
    half_width_mm: f64,
    end_pad_mm: f64,
    color: Color32,
    pixels_per_point: f32,
) -> egui::Mesh {
    let n = pts.len();
    let mut mesh = egui::Mesh::default();
    if n < 2 {
        return mesh;
    }
    let ppp = pixels_per_point.max(1.0);
    let feather_mm = (0.90 / (camera.zoom.max(0.04) * ppp)) as f64;
    let f_mm = feather_mm.min(half_width_mm * 0.45);
    mesh.vertices.reserve(n * 4);
    mesh.indices.reserve((n - 1) * 18);

    for (i, &p) in pts.iter().enumerate() {
        let (nx, ny, mscale) = polyline_miter_normal_2d(pts, i);
        let tx = ny;
        let ty = -nx;
        let pad = if i == 0 {
            -end_pad_mm
        } else if i + 1 == n {
            end_pad_mm
        } else {
            0.0
        };
        let px = p.x + tx * pad;
        let py = p.y + ty * pad;
        let hw_in = (half_width_mm - f_mm * 0.45).max(0.0) * mscale;
        let hw_out = (half_width_mm + f_mm * 0.55) * mscale;

        let out_l = camera.world_to_screen(rect, px + nx * hw_out, py + ny * hw_out);
        let in_l = camera.world_to_screen(rect, px + nx * hw_in, py + ny * hw_in);
        let in_r = camera.world_to_screen(rect, px - nx * hw_in, py - ny * hw_in);
        let out_r = camera.world_to_screen(rect, px - nx * hw_out, py - ny * hw_out);

        let base = (i * 4) as u32;
        mesh.vertices.push(egui::epaint::Vertex {
            pos: out_l,
            uv: egui::epaint::WHITE_UV,
            color: Color32::TRANSPARENT,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: in_l,
            uv: egui::epaint::WHITE_UV,
            color,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: in_r,
            uv: egui::epaint::WHITE_UV,
            color,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: out_r,
            uv: egui::epaint::WHITE_UV,
            color: Color32::TRANSPARENT,
        });

        if i + 1 < n {
            let next = base + 4;
            mesh.indices.extend_from_slice(&[
                // Left AA fringe
                base,
                base + 1,
                next + 1,
                base,
                next + 1,
                next,
                // Core solid fill
                base + 1,
                base + 2,
                next + 2,
                base + 1,
                next + 2,
                next + 1,
                // Right AA fringe
                base + 2,
                base + 3,
                next + 3,
                base + 2,
                next + 3,
                next + 2,
            ]);
        }
    }
    mesh
}

pub(super) fn draw_mitered_ribbon_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    pts: &[Vec2],
    half_width_mm: f64,
    end_pad_mm: f64,
    color: Color32,
) {
    if pts.len() < 2 {
        return;
    }
    let mesh = build_feathered_ribbon_mesh_2d(
        rect,
        camera,
        pts,
        half_width_mm,
        end_pad_mm,
        color,
        painter.pixels_per_point(),
    );
    painter.add(egui::Shape::mesh(mesh));
}

pub(super) fn piece_branches_world(piece: &TrackPiece, entry: Option<&CatalogEntry>) -> Vec<Vec<Vec2>> {
    if let Some(e) = entry {
        let to_w = |lx: f64, ly: f64| -> Vec2 {
            let rp = rotate(Vec2 { x: lx, y: ly }, piece.transform.rotation);
            Vec2 {
                x: piece.transform.x + rp.x,
                y: piece.transform.y + rp.y,
            }
        };
        if e.kind == "turntable" {
            let outer_r = crate::turntables_3d::turntable_outer_radius(piece, entry);
            let scale_f = (outer_r / 189.0).clamp(0.4, 2.5);
            let pit_r = 139.0 * scale_f;
            let slot_angles: Vec<f64> = e
                .local_connectors
                .as_ref()
                .filter(|lc| !lc.is_empty())
                .map(|lc| lc.iter().map(|c| c.angle).collect())
                .unwrap_or_else(|| (0..30).map(|i| i as f64 * 12.0).collect());
            let mut out = Vec::with_capacity(1 + slot_angles.len());
            out.push(vec![to_w(-pit_r, 0.0), to_w(pit_r, 0.0)]);
            for deg in slot_angles {
                let rad = deg.to_radians();
                let (ca, sa) = (rad.cos(), rad.sin());
                out.push(vec![to_w(pit_r * ca, pit_r * sa), to_w(outer_r * ca, outer_r * sa)]);
            }
            return out;
        }
        if e.kind == "transfer-table" {
            let l = e.length.unwrap_or(360.0);
            let pit_left = 36.0;
            let pit_right = l - 36.0;
            let track_zs = crate::turntables_3d::extract_transfer_track_zs(entry);
            let bridge_z = track_zs[track_zs.len() / 2] as f64;
            let mut out = Vec::with_capacity(1 + track_zs.len() * 2);
            out.push(vec![to_w(pit_left, bridge_z), to_w(pit_right, bridge_z)]);
            for &tzf in &track_zs {
                let tz = tzf as f64;
                out.push(vec![to_w(0.0, tz), to_w(pit_left, tz)]);
                out.push(vec![to_w(pit_right, tz), to_w(l, tz)]);
            }
            return out;
        }
    }

    // 1. Flextrack spline
    if let (Some(offsets), Some(chord)) = (&piece.custom_offsets, piece.custom_chord_dist) {
        if offsets.len() >= 3 {
            let flat = sample_flex_spline_points(chord, offsets[0], offsets[1], offsets[2], 48);
            let mut pts = Vec::with_capacity(49);
            for i in (0..flat.len()).step_by(5) {
                let rp = rotate(Vec2 { x: flat[i], y: flat[i + 1] }, piece.transform.rotation);
                pts.push(Vec2 {
                    x: piece.transform.x + rp.x,
                    y: piece.transform.y + rp.y,
                });
            }
            return vec![pts];
        }
    }

    let c = &piece.connectors;
    match c.len() {
        0 => vec![],
        1 => {
            // Buffer stop (Prellbock): stub from c[0] using catalog length (e.g. 77.5 mm for 24977/24978)
            let stub_len = piece
                .custom_length
                .or_else(|| entry.and_then(|e| e.length))
                .unwrap_or(70.0)
                .max(18.0);
            let rad = (c[0].angle + 180.0).to_radians();
            vec![vec![
                Vec2 { x: c[0].x, y: c[0].y },
                Vec2 { x: c[0].x + rad.cos() * stub_len, y: c[0].y + rad.sin() * stub_len },
            ]]
        }
        2 => vec![sample_arc_between_connectors(
            c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 40,
        )],
        3 => vec![
            sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 36),
            sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[2].x, c[2].y, c[2].angle, 36),
        ],
        4 => {
            // Distinguish 3-way turnout (1 entry + 3 exits facing away from c0)
            // vs crossing / double-slip (2 opposite pairs).
            let a0 = c[0].angle.to_radians();
            let v0x = a0.cos();
            let v0y = a0.sin();
            let facing_opposite = c[1..4]
                .iter()
                .filter(|ci| {
                    let ai = ci.angle.to_radians();
                    v0x * ai.cos() + v0y * ai.sin() < -0.15
                })
                .count();
            if facing_opposite == 3 {
                // 3-way turnout
                vec![
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 36),
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[2].x, c[2].y, c[2].angle, 36),
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[3].x, c[3].y, c[3].angle, 36),
                ]
            } else {
                // Crossing or double-slip
                vec![
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 36),
                    sample_arc_between_connectors(c[2].x, c[2].y, c[2].angle, c[3].x, c[3].y, c[3].angle, 36),
                ]
            }
        }
        _ => {
            // Turntable: bridge across center
            let mut out = Vec::new();
            let half = c.len() / 2;
            out.push(vec![
                Vec2 { x: c[0].x, y: c[0].y },
                Vec2 { x: c[half].x, y: c[half].y },
            ]);
            out
        }
    }
}

pub(super) fn is_layer_visible(layout: &Layout, layer_id: Option<&str>) -> bool {
    let lid = layer_id.unwrap_or("layer-1");
    layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == lid))
        .map(|l| l.visible)
        .unwrap_or(true)
}

pub(super) fn is_layer_editable(layout: &Layout, layer_id: Option<&str>) -> bool {
    let lid = layer_id.unwrap_or("layer-1");
    layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == lid))
        .map(|l| l.visible && !l.locked)
        .unwrap_or(true)
}

pub fn is_layer_editable_with_protection(
    layout: &Layout,
    layer_id: Option<&str>,
    layer_protection: bool,
) -> bool {
    let lid = layer_id.unwrap_or("layer-1");
    if layer_protection {
        let active_lid = layout.active_layer_id.as_deref().unwrap_or("layer-1");
        if lid != active_lid {
            return false;
        }
    }
    is_layer_editable(layout, layer_id)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_model_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    m: &Model3D,
    top_mesh: Option<&CachedModelTopMesh2D>,
    is_sel: bool,
    is_active_layer: bool,
    show_labels: bool,
) {
    let scale = m.scale.unwrap_or(1.0);
    let hw = m.width * scale * 0.5;
    let hd = m.depth * scale * 0.5;
    let rot = m.transform.rotation;
    let corners = [
        Vec2 { x: -hw, y: -hd },
        Vec2 { x: hw, y: -hd },
        Vec2 { x: hw, y: hd },
        Vec2 { x: -hw, y: hd },
    ];
    let spts: Vec<Pos2> = corners
        .iter()
        .map(|c| {
            let rc = rotate(*c, rot);
            camera.world_to_screen(rect, m.transform.x + rc.x, m.transform.y + rc.y)
        })
        .collect();

    let is_mtn = m.builtin_id.as_deref() == Some("mountain");
    let is_ground = m.builtin_id.as_deref() == Some("ground-terrain");
    let g_type = m.ground_terrain_type.as_deref().unwrap_or("hill");
    let is_rail_bldg = matches!(
        m.builtin_id.as_deref(),
        Some("engine-shed")
            | Some("turntable-trainhall")
            | Some("coaling-station")
            | Some("diesel-fuel-station")
            | Some("buffer-stop")
            | Some("platform")
            | Some("bahnsteig")
    );
    let is_industrial = matches!(
        m.builtin_id.as_deref(),
        Some("industry-building") | Some("skyscraper")
    );
    let is_custom_glb = m.file_data.is_some() || matches!(m.model_type.as_str(), "glb" | "gltf");

    let sel_stroke = Color32::from_rgb(56, 189, 248);

    if is_ground {
        let (mut outer_col, mut inner_col, mut stroke_col) = match g_type {
            "water" => (
                Color32::from_rgba_unmultiplied(2, 132, 199, 115),
                Color32::from_rgba_unmultiplied(56, 189, 248, 130),
                Color32::from_rgb(3, 105, 161),
            ),
            "sand" => (
                Color32::from_rgba_unmultiplied(214, 185, 124, 132),
                Color32::from_rgba_unmultiplied(231, 211, 163, 145),
                Color32::from_rgb(161, 130, 74),
            ),
            "dirt" => (
                Color32::from_rgba_unmultiplied(107, 76, 48, 132),
                Color32::from_rgba_unmultiplied(133, 96, 62, 145),
                Color32::from_rgb(74, 51, 31),
            ),
            _ => (
                Color32::from_rgba_unmultiplied(77, 124, 15, 122),
                Color32::from_rgba_unmultiplied(101, 163, 13, 135),
                Color32::from_rgb(54, 83, 20),
            ),
        };
        if is_sel {
            stroke_col = sel_stroke;
        } else if !is_active_layer {
            outer_col = desaturate_color(outer_col, 0.78, 0.72);
            inner_col = desaturate_color(inner_col, 0.78, 0.72);
            stroke_col = desaturate_color(stroke_col, 0.78, 0.72);
        }

        if let Some(nodes) = m.mountain_path.as_ref().filter(|p| !p.is_empty()) {
            let avg_r = (nodes.iter().map(|n| n.radius).sum::<f64>() / nodes.len() as f64) * scale;
            let pts_scr: Vec<Pos2> = nodes
                .iter()
                .map(|n| {
                    let rp = rotate(Vec2 { x: n.x * scale, y: n.y * scale }, rot);
                    camera.world_to_screen(rect, m.transform.x + rp.x, m.transform.y + rp.y)
                })
                .collect();
            if pts_scr.len() >= 2 {
                let w_outer = (avg_r as f32 * 2.0 * camera.zoom).max(8.0);
                let w_inner = (avg_r as f32 * 1.3 * camera.zoom).max(4.0);
                painter.add(egui::Shape::line(pts_scr.clone(), Stroke::new(w_outer, outer_col)));
                painter.add(egui::Shape::line(pts_scr, Stroke::new(w_inner, inner_col)));
            } else {
                let r_scr = ((nodes[0].radius * scale) as f32 * camera.zoom).max(8.0);
                painter.circle_filled(pts_scr[0], r_scr, outer_col);
                painter.circle_filled(pts_scr[0], r_scr * 0.65, inner_col);
            }
            if is_sel {
                painter.add(egui::Shape::closed_line(spts, Stroke::new(2.0_f32, sel_stroke)));
            }
        } else {
            let ellipse_pts = |sx: f64, sy: f64| -> Vec<Pos2> {
                let steps = 56usize;
                (0..steps)
                    .map(|k| {
                        let a = (k as f64 / steps as f64) * std::f64::consts::TAU;
                        let rp = rotate(Vec2 { x: hw * sx * a.cos(), y: hd * sy * a.sin() }, rot);
                        camera.world_to_screen(rect, m.transform.x + rp.x, m.transform.y + rp.y)
                    })
                    .collect()
            };
            painter.add(egui::Shape::convex_polygon(
                ellipse_pts(1.0, 1.0),
                outer_col,
                Stroke::new(if is_sel { 2.4_f32 } else { 1.4_f32 }, stroke_col),
            ));
            painter.add(egui::Shape::convex_polygon(
                ellipse_pts(0.68, 0.68),
                inner_col,
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(stroke_col.r(), stroke_col.g(), stroke_col.b(), 115)),
            ));
            if g_type == "water" {
                for ry in [-0.25, 0.0, 0.25] {
                    let p0 = rotate(Vec2 { x: -hw * 0.35, y: hd * ry }, rot);
                    let p1 = rotate(Vec2 { x: hw * 0.35, y: hd * ry }, rot);
                    painter.line_segment(
                        [
                            camera.world_to_screen(rect, m.transform.x + p0.x, m.transform.y + p0.y),
                            camera.world_to_screen(rect, m.transform.x + p1.x, m.transform.y + p1.y),
                        ],
                        Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(224, 242, 254, 145)),
                    );
                }
            }
        }
    } else if is_mtn {
        let style = m.mountain_style.as_deref().unwrap_or("alpine");
        let (mut c_base, mut c_mid, mut c_upper, mut c_peak) = match style {
            "forest" => (
                Color32::from_rgba_unmultiplied(20, 83, 45, 170),
                Color32::from_rgba_unmultiplied(22, 101, 52, 185),
                Color32::from_rgba_unmultiplied(87, 83, 78, 198),
                Color32::from_rgba_unmultiplied(168, 162, 158, 224),
            ),
            "meadow" => (
                Color32::from_rgba_unmultiplied(54, 83, 20, 170),
                Color32::from_rgba_unmultiplied(77, 124, 15, 185),
                Color32::from_rgba_unmultiplied(101, 163, 13, 198),
                Color32::from_rgba_unmultiplied(132, 204, 22, 224),
            ),
            "rocky" => (
                Color32::from_rgba_unmultiplied(41, 37, 36, 174),
                Color32::from_rgba_unmultiplied(68, 64, 60, 185),
                Color32::from_rgba_unmultiplied(120, 113, 108, 200),
                Color32::from_rgba_unmultiplied(214, 211, 209, 226),
            ),
            _ => (
                Color32::from_rgba_unmultiplied(41, 37, 36, 174),
                Color32::from_rgba_unmultiplied(68, 64, 60, 185),
                Color32::from_rgba_unmultiplied(120, 113, 108, 200),
                Color32::from_rgba_unmultiplied(248, 250, 252, 228),
            ),
        };
        let mut border_col = if is_sel { sel_stroke } else { Color32::from_rgb(68, 64, 60) };
        if !is_active_layer && !is_sel {
            c_base = desaturate_color(c_base, 0.78, 0.72);
            c_mid = desaturate_color(c_mid, 0.78, 0.72);
            c_upper = desaturate_color(c_upper, 0.78, 0.72);
            c_peak = desaturate_color(c_peak, 0.78, 0.72);
            border_col = desaturate_color(border_col, 0.78, 0.72);
        }

        if let Some(nodes) = m.mountain_path.as_ref().filter(|p| !p.is_empty()) {
            let avg_r = (nodes.iter().map(|n| n.radius).sum::<f64>() / nodes.len() as f64) * scale;
            let tiers = [(1.0_f32, c_base), (0.68, c_mid), (0.38, c_upper), (0.18, c_peak)];
            if nodes.len() >= 2 {
                let pts_scr: Vec<Pos2> = nodes
                    .iter()
                    .map(|n| {
                        let rp = rotate(Vec2 { x: n.x * scale, y: n.y * scale }, rot);
                        camera.world_to_screen(rect, m.transform.x + rp.x, m.transform.y + rp.y)
                    })
                    .collect();
                for (t_sc, t_col) in tiers {
                    let sw = (avg_r as f32 * 2.0 * t_sc * camera.zoom).max(4.0);
                    painter.add(egui::Shape::line(pts_scr.clone(), Stroke::new(sw, t_col)));
                }
            } else {
                let n0 = &nodes[0];
                let rp = rotate(Vec2 { x: n0.x * scale, y: n0.y * scale }, rot);
                let sp = camera.world_to_screen(rect, m.transform.x + rp.x, m.transform.y + rp.y);
                for (t_sc, t_col) in tiers {
                    let r_scr = ((n0.radius * scale) as f32 * t_sc * camera.zoom).max(4.0);
                    painter.circle_filled(sp, r_scr, t_col);
                }
            }
            if is_sel {
                painter.add(egui::Shape::closed_line(spts, Stroke::new(2.0_f32, sel_stroke)));
            }
        } else {
            let ellipse_pts = |sx: f64, sy: f64| -> Vec<Pos2> {
                let steps = 56usize;
                (0..steps)
                    .map(|k| {
                        let a = (k as f64 / steps as f64) * std::f64::consts::TAU;
                        let rp = rotate(Vec2 { x: hw * sx * a.cos(), y: hd * sy * a.sin() }, rot);
                        camera.world_to_screen(rect, m.transform.x + rp.x, m.transform.y + rp.y)
                    })
                    .collect()
            };
            painter.add(egui::Shape::convex_polygon(
                ellipse_pts(1.0, 1.0),
                c_base,
                Stroke::new(if is_sel { 2.4_f32 } else { 1.5_f32 }, border_col),
            ));
            painter.add(egui::Shape::convex_polygon(ellipse_pts(0.68, 0.68), c_mid, Stroke::NONE));
            painter.add(egui::Shape::convex_polygon(ellipse_pts(0.38, 0.38), c_upper, Stroke::NONE));
            painter.add(egui::Shape::convex_polygon(
                ellipse_pts(0.18, 0.18),
                c_peak,
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(226, 232, 240, 200)),
            ));
            let cx0 = rotate(Vec2 { x: -hw * 0.25, y: 0.0 }, rot);
            let cx1 = rotate(Vec2 { x: hw * 0.25, y: 0.0 }, rot);
            let cy0 = rotate(Vec2 { x: 0.0, y: -hd * 0.22 }, rot);
            let cy1 = rotate(Vec2 { x: 0.0, y: hd * 0.22 }, rot);
            let x_col = Color32::from_rgba_unmultiplied(255, 255, 255, 210);
            painter.line_segment(
                [
                    camera.world_to_screen(rect, m.transform.x + cx0.x, m.transform.y + cx0.y),
                    camera.world_to_screen(rect, m.transform.x + cx1.x, m.transform.y + cx1.y),
                ],
                Stroke::new(1.3_f32, x_col),
            );
            painter.line_segment(
                [
                    camera.world_to_screen(rect, m.transform.x + cy0.x, m.transform.y + cy0.y),
                    camera.world_to_screen(rect, m.transform.x + cy1.x, m.transform.y + cy1.y),
                ],
                Stroke::new(1.3_f32, x_col),
            );
        }
    } else if let Some(tm) = top_mesh.filter(|tm| !tm.tris.is_empty()) {
        // Authentic orthographic top-down projection of the 3D viewer mesh
        let rad = rot.to_radians();
        let cos_r = rad.cos();
        let sin_r = rad.sin();
        let mut mesh = egui::Mesh::default();
        mesh.vertices.reserve(tm.tris.len() * 3);
        mesh.indices.reserve(tm.tris.len() * 3);

        for (pts, cols) in &tm.tris {
            let base = mesh.vertices.len() as u32;
            for k in 0..3 {
                let lx = pts[k][0] as f64;
                let ly = pts[k][1] as f64;
                let wx = m.transform.x + lx * cos_r - ly * sin_r;
                let wy = m.transform.y + lx * sin_r + ly * cos_r;
                let mut col = cols[k];
                if !is_active_layer && !is_sel {
                    col = desaturate_color(col, 0.75, 0.72);
                } else if is_sel {
                    col = brighten_color(col, 0.12);
                }
                mesh.colored_vertex(camera.world_to_screen(rect, wx, wy), col);
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        painter.add(egui::Shape::mesh(mesh));

        if is_sel {
            painter.add(egui::Shape::closed_line(
                spts,
                Stroke::new(2.2_f32, Color32::from_rgb(56, 189, 248)),
            ));
        }
    } else {
        let mut fill = if is_sel {
            Color32::from_rgba_unmultiplied(96, 210, 255, 130)
        } else if is_custom_glb {
            Color32::from_rgba_unmultiplied(99, 102, 241, 130)
        } else if is_rail_bldg {
            Color32::from_rgba_unmultiplied(156, 92, 68, 135)
        } else if is_industrial {
            Color32::from_rgba_unmultiplied(100, 116, 139, 150)
        } else {
            Color32::from_rgba_unmultiplied(148, 163, 184, 145)
        };
        let mut stroke_col = if is_sel {
            Color32::from_rgb(125, 224, 255)
        } else if is_custom_glb {
            Color32::from_rgb(165, 180, 252)
        } else if is_rail_bldg {
            Color32::from_rgb(226, 164, 132)
        } else if is_industrial {
            Color32::from_rgb(226, 232, 240)
        } else {
            Color32::from_rgb(203, 213, 225)
        };
        if !is_active_layer && !is_sel {
            fill = desaturate_color(fill, 0.78, 0.72);
            stroke_col = desaturate_color(stroke_col, 0.78, 0.72);
        }
        let stroke = Stroke::new(if is_sel { 2.2_f32 } else { 1.4_f32 }, stroke_col);
        painter.add(egui::Shape::convex_polygon(spts, fill, stroke));
    }

    if show_labels && (is_sel || (hw.max(hd) as f32) * camera.zoom >= 8.0) {
        let center = camera.world_to_screen(rect, m.transform.x, m.transform.y);
        if rect.expand(48.0).contains(center) {
            let font_sz = (10.5 * camera.zoom.sqrt()).clamp(8.0, 12.0);
            let txt_col = if is_active_layer || is_sel {
                Color32::WHITE
            } else {
                Color32::from_rgba_unmultiplied(203, 213, 225, 165)
            };
            let galley = painter.layout_no_wrap(m.name.clone(), FontId::proportional(font_sz), txt_col);
            let pad = EguiVec2::new(4.0, 1.5);
            let pill = Rect::from_center_size(center, galley.size() + pad * 2.0);
            painter.rect_filled(
                pill,
                4.0,
                Color32::from_rgba_unmultiplied(15, 23, 42, if is_active_layer || is_sel { 205 } else { 135 }),
            );
            painter.galley(pill.min + pad, galley, txt_col);
        }
    }
}

pub(super) fn model_contains_point(m: &Model3D, wp: Vec2) -> bool {
    let scale = m.scale.unwrap_or(1.0);
    if matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain"))
        && m.mountain_path.as_ref().map_or(false, |p| !p.is_empty())
    {
        let (lx, lz) = world_to_mountain_local(wp.x, wp.y, m);
        let (norm_r, _) = evaluate_mountain_local(lx, lz, m);
        return norm_r <= 1.0;
    }
    let local = rotate(
        Vec2 {
            x: wp.x - m.transform.x,
            y: wp.y - m.transform.y,
        },
        -m.transform.rotation,
    );
    let hw = (m.width * scale * 0.5).max(15.0);
    let hd = (m.depth * scale * 0.5).max(15.0);
    local.x.abs() <= hw && local.y.abs() <= hd
}

