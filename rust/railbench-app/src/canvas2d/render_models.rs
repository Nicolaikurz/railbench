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
            // vs double-slip (4 branches) vs plain crossing (2 opposite pairs).
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
            let is_three_way = entry.map_or(false, |e| e.kind == "three-way") || facing_opposite == 3;
            if is_three_way {
                // 3-way turnout
                vec![
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 36),
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[2].x, c[2].y, c[2].angle, 36),
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[3].x, c[3].y, c[3].angle, 36),
                ]
            } else if entry.map_or(false, |e| e.kind == "double-slip") {
                // Double slip: 2 crossing straights + 2 curved slip branches
                vec![
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 36),
                    sample_arc_between_connectors(c[2].x, c[2].y, c[2].angle, c[3].x, c[3].y, c[3].angle, 36),
                    sample_arc_between_connectors(c[0].x, c[0].y, c[0].angle, c[3].x, c[3].y, c[3].angle, 36),
                    sample_arc_between_connectors(c[2].x, c[2].y, c[2].angle, c[1].x, c[1].y, c[1].angle, 36),
                ]
            } else {
                // Plain crossing
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
    mtn_map: Option<&CachedMountainMap2D>,
    ground_map: Option<&CachedGroundMap2D>,
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
        let (mut outer_col, mut mid_col, mut inner_col, mut stroke_col) = match g_type {
            "water" => (
                Color32::from_rgba_unmultiplied(14, 165, 233, 54),
                Color32::from_rgba_unmultiplied(2, 132, 199, 64),
                Color32::from_rgba_unmultiplied(3, 105, 161, 74),
                Color32::from_rgba_unmultiplied(56, 189, 248, 195),
            ),
            "sand" => (
                Color32::from_rgba_unmultiplied(214, 185, 124, 88),
                Color32::from_rgba_unmultiplied(226, 202, 148, 96),
                Color32::from_rgba_unmultiplied(237, 219, 173, 104),
                Color32::from_rgb(161, 130, 74),
            ),
            "dirt" => (
                Color32::from_rgba_unmultiplied(107, 76, 48, 88),
                Color32::from_rgba_unmultiplied(124, 89, 57, 96),
                Color32::from_rgba_unmultiplied(142, 103, 68, 104),
                Color32::from_rgb(92, 64, 40),
            ),
            _ => (
                Color32::from_rgba_unmultiplied(77, 124, 15, 76),
                Color32::from_rgba_unmultiplied(92, 148, 18, 84),
                Color32::from_rgba_unmultiplied(112, 174, 24, 92),
                Color32::from_rgb(77, 124, 15),
            ),
        };
        if is_sel {
            stroke_col = sel_stroke;
        } else if !is_active_layer {
            outer_col = desaturate_color(outer_col, 0.78, 0.72);
            mid_col = desaturate_color(mid_col, 0.78, 0.72);
            inner_col = desaturate_color(inner_col, 0.78, 0.72);
            stroke_col = desaturate_color(stroke_col, 0.78, 0.72);
        }

        if let Some(gm) = ground_map {
            let band_cols = [outer_col, mid_col, inner_col];
            if !gm.quads.is_empty() {
                let mut mesh = egui::Mesh::default();
                mesh.vertices.reserve(gm.quads.len() * 4);
                mesh.indices.reserve(gm.quads.len() * 6);
                for &(pts, band) in &gm.quads {
                    let col = band_cols[(band as usize).min(2)];
                    let base = mesh.vertices.len() as u32;
                    for pt in pts {
                        mesh.vertices.push(egui::epaint::Vertex {
                            pos: camera.world_to_screen(rect, pt.x, pt.y),
                            uv: egui::epaint::WHITE_UV,
                            color: col,
                        });
                    }
                    mesh.indices
                        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                }
                painter.add(egui::Shape::mesh(mesh));
            }
            let edge_stroke = Stroke::new(if is_sel { 2.2_f32 } else { 1.4_f32 }, stroke_col);
            for &(a, b) in &gm.contour_segs {
                painter.line_segment(
                    [
                        camera.world_to_screen(rect, a.x, a.y),
                        camera.world_to_screen(rect, b.x, b.y),
                    ],
                    edge_stroke,
                );
            }
            if g_type == "water" && !gm.ripple_segs.is_empty() {
                let ripple_col = Color32::from_rgba_unmultiplied(224, 242, 254, 155);
                let ripple_stroke = Stroke::new(1.2_f32, ripple_col);
                for &(a, b) in &gm.ripple_segs {
                    painter.line_segment(
                        [
                            camera.world_to_screen(rect, a.x, a.y),
                            camera.world_to_screen(rect, b.x, b.y),
                        ],
                        ripple_stroke,
                    );
                }
            }
            if is_sel {
                painter.add(egui::Shape::closed_line(spts, Stroke::new(1.8_f32, sel_stroke)));
            }
        }
    } else if is_mtn {
        // Authentic Swiss/German Topographic Map (TK25 / Swisstopo) rendering:
        // ultra-light translucent hypsometric wash + real Marching-Squares elevation contours
        // + downhill slope hachures (Schraffen) + summit triangulation marker (▲ height)
        let style = m.mountain_style.as_deref().unwrap_or("alpine");
        let (mut band_cols, mut idx_col, mut sub_col, mut hach_col, mut peak_col) = match style {
            "forest" => (
                [
                    Color32::from_rgba_unmultiplied(34, 139, 68, 22),
                    Color32::from_rgba_unmultiplied(46, 152, 78, 30),
                    Color32::from_rgba_unmultiplied(116, 138, 92, 36),
                    Color32::from_rgba_unmultiplied(186, 178, 158, 44),
                ],
                Color32::from_rgba_unmultiplied(120, 92, 56, 205),
                Color32::from_rgba_unmultiplied(120, 92, 56, 120),
                Color32::from_rgba_unmultiplied(110, 84, 52, 145),
                Color32::from_rgb(241, 245, 249),
            ),
            "meadow" => (
                [
                    Color32::from_rgba_unmultiplied(101, 163, 13, 20),
                    Color32::from_rgba_unmultiplied(132, 184, 42, 28),
                    Color32::from_rgba_unmultiplied(168, 198, 76, 34),
                    Color32::from_rgba_unmultiplied(214, 218, 126, 40),
                ],
                Color32::from_rgba_unmultiplied(146, 108, 52, 200),
                Color32::from_rgba_unmultiplied(146, 108, 52, 118),
                Color32::from_rgba_unmultiplied(138, 100, 48, 140),
                Color32::from_rgb(241, 245, 249),
            ),
            "rocky" => (
                [
                    Color32::from_rgba_unmultiplied(120, 113, 108, 22),
                    Color32::from_rgba_unmultiplied(148, 140, 134, 30),
                    Color32::from_rgba_unmultiplied(180, 174, 168, 38),
                    Color32::from_rgba_unmultiplied(218, 214, 210, 46),
                ],
                Color32::from_rgba_unmultiplied(168, 152, 136, 210),
                Color32::from_rgba_unmultiplied(156, 142, 128, 125),
                Color32::from_rgba_unmultiplied(148, 134, 120, 150),
                Color32::from_rgb(248, 250, 252),
            ),
            _ => (
                // Classic Swisstopo / Alpine sepia-ochre contour map with snow peak tint
                [
                    Color32::from_rgba_unmultiplied(161, 132, 92, 20),
                    Color32::from_rgba_unmultiplied(148, 126, 98, 28),
                    Color32::from_rgba_unmultiplied(148, 163, 184, 36),
                    Color32::from_rgba_unmultiplied(241, 245, 249, 48),
                ],
                Color32::from_rgba_unmultiplied(180, 126, 74, 215),
                Color32::from_rgba_unmultiplied(176, 124, 74, 125),
                Color32::from_rgba_unmultiplied(168, 118, 70, 155),
                Color32::from_rgb(248, 250, 252),
            ),
        };
        if is_sel {
            idx_col = sel_stroke;
            sub_col = Color32::from_rgba_unmultiplied(56, 189, 248, 165);
        } else if !is_active_layer {
            for c in &mut band_cols {
                *c = desaturate_color(*c, 0.78, 0.72);
            }
            idx_col = desaturate_color(idx_col, 0.78, 0.72);
            sub_col = desaturate_color(sub_col, 0.78, 0.72);
            hach_col = desaturate_color(hach_col, 0.78, 0.72);
            peak_col = desaturate_color(peak_col, 0.78, 0.72);
        }

        if let Some(map) = mtn_map {
            // 1. Single batched translucent hypsometric wash mesh (fast & never hides tracks underneath)
            if !map.tint_quads.is_empty() {
                let mut mesh = egui::Mesh::default();
                mesh.vertices.reserve(map.tint_quads.len() * 4);
                mesh.indices.reserve(map.tint_quads.len() * 6);
                for &(pts, band) in &map.tint_quads {
                    let col = band_cols[(band as usize).min(3)];
                    let base = mesh.vertices.len() as u32;
                    for pt in pts {
                        let sp = camera.world_to_screen(rect, pt.x, pt.y);
                        mesh.vertices.push(egui::epaint::Vertex {
                            pos: sp,
                            uv: egui::epaint::WHITE_UV,
                            color: col,
                        });
                    }
                    mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                }
                painter.add(egui::Shape::mesh(mesh));
            }

            // 2. Cartographic downhill slope hachures (Schraffen)
            let hach_stroke = Stroke::new((0.9_f32 * camera.zoom.sqrt()).clamp(0.7, 1.3), hach_col);
            for &(p0, p1) in &map.hachures {
                painter.line_segment(
                    [
                        camera.world_to_screen(rect, p0.x, p0.y),
                        camera.world_to_screen(rect, p1.x, p1.y),
                    ],
                    hach_stroke,
                );
            }

            // 3. Topographic contour lines: fine intermediate contours + bolder index contours (Haupthoehenlinien)
            let sub_stroke = Stroke::new((0.85_f32 * camera.zoom.sqrt()).clamp(0.65, 1.25), sub_col);
            let idx_stroke = Stroke::new(
                if is_sel { 2.0_f32 } else { (1.45_f32 * camera.zoom.sqrt()).clamp(1.1, 2.1) },
                idx_col,
            );
            for &(p0, p1, is_index) in &map.contour_segs {
                painter.line_segment(
                    [
                        camera.world_to_screen(rect, p0.x, p0.y),
                        camera.world_to_screen(rect, p1.x, p1.y),
                    ],
                    if is_index { idx_stroke } else { sub_stroke },
                );
            }

            // 4. Cartographic summit triangulation markers (▲) + elevation height labels
            let tri_r = (5.2_f32 * camera.zoom.sqrt()).clamp(3.6, 7.5);
            for &(pk, ph) in &map.peaks {
                let sp = camera.world_to_screen(rect, pk.x, pk.y);
                let tri = vec![
                    Pos2::new(sp.x, sp.y - tri_r),
                    Pos2::new(sp.x + tri_r * 0.88, sp.y + tri_r * 0.65),
                    Pos2::new(sp.x - tri_r * 0.88, sp.y + tri_r * 0.65),
                ];
                painter.add(egui::Shape::convex_polygon(
                    tri,
                    peak_col,
                    Stroke::new(1.1_f32, idx_col),
                ));
                if show_labels && camera.zoom >= 0.22 {
                    painter.text(
                        Pos2::new(sp.x + tri_r + 4.0, sp.y - 1.0),
                        egui::Align2::LEFT_CENTER,
                        format!("{:.0} mm", ph),
                        FontId::monospace(9.5),
                        peak_col,
                    );
                }
            }
            if is_sel {
                painter.add(egui::Shape::closed_line(spts, Stroke::new(1.8_f32, sel_stroke)));
            }
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

