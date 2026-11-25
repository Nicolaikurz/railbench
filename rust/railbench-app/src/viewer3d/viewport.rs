use super::*;

#[allow(clippy::too_many_arguments)]
pub fn draw_viewport_3d(
    ui: &mut egui::Ui,
    camera: &mut Camera3D,
    cached_bundle: &Option<Scene3DBundle>,
    trains_mesh: Option<(u64, Arc<SceneMesh3D>)>,
    train_mode_active: &mut bool,
    layout: &Layout,
    env_mode: &mut Viewer3DEnv,
    day_night_active: &mut bool,
    time_of_day: &mut f32,
    day_night_speed: &mut f32,
    show_wood_plates: &mut bool,
    first_short_circuit: Option<Vec2>,
    dark_mode: bool,
    lang: Lang,
    dt_secs: f32,
) {
    if *day_night_active {
        let delta_hours = dt_secs * (24.0 / 1200.0) * (*day_night_speed);
        *time_of_day = (*time_of_day + delta_hours).rem_euclid(24.0);
    }

    let avail = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());

    // Full 360° / 180° Orbit (left drag, including underneath table)
    if response.dragged_by(egui::PointerButton::Primary) {
        let d = response.drag_delta();
        camera.yaw -= d.x * 0.0065;
        camera.pitch = (camera.pitch + d.y * 0.0065).clamp(-1.48, 1.48);
    }
    // Pan (right or middle drag)
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        let d = response.drag_delta();
        let speed = camera.distance * 0.0012;
        let rx = camera.yaw.cos();
        let rz = -camera.yaw.sin();
        let fx = -camera.yaw.sin();
        let fz = -camera.yaw.cos();
        camera.target[0] -= (rx * d.x - fx * d.y) * speed;
        camera.target[2] -= (rz * d.x - fz * d.y) * speed;
    }
    // Zoom (scroll)
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.01 {
            let factor = (1.0 - scroll * 0.0018).clamp(0.75, 1.35);
            camera.distance = (camera.distance * factor).clamp(60.0, 25000.0);
        }
    }

    let ppp = ui.ctx().pixels_per_point();
    let px_w = ((rect.width() * ppp).round() as u32).max(16);
    let px_h = ((rect.height() * ppp).round() as u32).max(16);
    let aspect = rect.width() / rect.height().max(1.0);
    let eye = camera.eye();
    let sky = eval_sky_state(*time_of_day, *env_mode, dark_mode);

    let uniforms = Uniforms3D {
        view_proj: camera.view_proj(aspect),
        sun_dir: sky.sun_dir,
        cam_pos: [eye[0], eye[1], eye[2], 1.0],
        fog_color: sky.fog_color,
        sun_color_int: sky.sun_color_int,
        hemi_sky_int: sky.hemi_sky_int,
        hemi_ground_pad: sky.hemi_ground_pad,
    };

    let cb = egui_wgpu::Callback::new_paint_callback(
        rect,
        Viewport3DCallback {
            uniforms,
            bundle: cached_bundle.clone(),
            trains_mesh,
            time_of_day: *time_of_day,
            env_mode: *env_mode,
            viewport_px: [px_w, px_h],
            clear_color: sky.clear_color,
        },
    );
    ui.painter().add(cb);

    let is_de = lang == Lang::De;
    let hud_bg = if dark_mode {
        Color32::from_rgba_unmultiplied(15, 23, 42, 215)
    } else {
        Color32::from_rgba_unmultiplied(248, 250, 252, 235)
    };
    let hud_border = if dark_mode {
        Color32::from_rgba_unmultiplied(148, 163, 184, 70)
    } else {
        Color32::from_rgba_unmultiplied(148, 163, 184, 150)
    };
    let hud_accent = if dark_mode {
        Color32::from_rgb(56, 189, 248)
    } else {
        Color32::from_rgb(2, 132, 199)
    };
    let hud_muted = if dark_mode {
        Color32::from_rgb(148, 163, 184)
    } else {
        Color32::from_rgb(71, 85, 105)
    };

    // Top-left 3D Controls Bar (Environment, Wood Plates toggle, Day/Night Cycle, Short Focus)
    egui::Area::new(egui::Id::new("hud_3d_top_bar"))
        .fixed_pos(rect.min + egui::vec2(14.0, 12.0))
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(hud_bg)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::symmetric(10, 6))
                .stroke(Stroke::new(1.0_f32, hud_border))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("3D").strong().size(11.5).color(hud_accent));
                        let env_lbl = match *env_mode {
                            Viewer3DEnv::HallLandscape => if is_de { "Halle & Landschaft" } else { "Hall & Landscape" },
                            Viewer3DEnv::Minimal => "CAD / Minimal",
                        };
                        if ui
                            .small_button(env_lbl)
                            .on_hover_text(if is_de {
                                "3D-Umgebung wechseln (Halle & Landschaft / CAD Minimal)"
                            } else {
                                "Toggle 3D Environment (Hall & Landscape / CAD Minimal)"
                            })
                            .clicked()
                        {
                            *env_mode = match *env_mode {
                                Viewer3DEnv::HallLandscape => Viewer3DEnv::Minimal,
                                Viewer3DEnv::Minimal => Viewer3DEnv::HallLandscape,
                            };
                        }

                        let wood_lbl = if is_de { "Holzplatten" } else { "Wood Plates" };
                        if ui.selectable_label(*show_wood_plates, wood_lbl).clicked() {
                            *show_wood_plates = !*show_wood_plates;
                        }
                        let tr_lbl = if is_de { "Zuege" } else { "Trains" };
                        if ui
                            .selectable_label(*train_mode_active, tr_lbl)
                            .on_hover_text(if is_de {
                                "Separaten 3D Zug-Modus ein/ausschalten (BR 243, BR 96 Mallet, V 60)"
                            } else {
                                "Toggle separate 3D Train Mode (BR 243, BR 96 Mallet, V 60)"
                            })
                            .clicked()
                        {
                            *train_mode_active = !*train_mode_active;
                        }

                        ui.separator();
                        let h = (*time_of_day).floor() as u32 % 24;
                        let m = (((*time_of_day) - (*time_of_day).floor()) * 60.0).floor() as u32 % 60;
                        let play_sym = if *day_night_active { "||" } else { ">" };
                        if ui.selectable_label(*day_night_active, format!("{play_sym} {h:02}:{m:02}")).clicked() {
                            *day_night_active = !*day_night_active;
                        }
                        ui.add_sized(
                            [88.0, 16.0],
                            egui::Slider::new(time_of_day, 0.0..=24.0).show_value(false),
                        );
                        if ui.small_button(format!("{}x", *day_night_speed as u32)).clicked() {
                            *day_night_speed = if (*day_night_speed - 1.0).abs() < 0.1 {
                                10.0
                            } else if (*day_night_speed - 10.0).abs() < 0.1 {
                                60.0
                            } else if (*day_night_speed - 60.0).abs() < 0.1 {
                                240.0
                            } else {
                                1.0
                            };
                        }

                        if let Some(sc) = first_short_circuit {
                            ui.separator();
                            if ui
                                .button(RichText::new("SHORT!").strong().color(Color32::from_rgb(248, 113, 113)))
                                .on_hover_text("Focus 3D camera on short circuit point")
                                .clicked()
                            {
                                camera.focus_point(sc.x, sc.y);
                            }
                        }

                        ui.label(
                            RichText::new("LMB: Orbit  •  RMB: Pan  •  Wheel: Zoom")
                                .size(10.5)
                                .color(hud_muted),
                        );
                    });
                });
        });

    // Interactive CAD ViewCube Widget below top-left HUD bar
    draw_viewcube_widget(ui, rect, camera, layout, is_de, dark_mode);
}

pub(super) fn draw_viewcube_widget(
    ui: &mut egui::Ui,
    viewport_rect: Rect,
    camera: &mut Camera3D,
    layout: &Layout,
    is_de: bool,
    dark_mode: bool,
) {
    let vc_bg = if dark_mode {
        Color32::from_rgba_unmultiplied(15, 23, 42, 212)
    } else {
        Color32::from_rgba_unmultiplied(248, 250, 252, 235)
    };
    let vc_border = if dark_mode {
        Color32::from_rgba_unmultiplied(148, 163, 184, 65)
    } else {
        Color32::from_rgba_unmultiplied(148, 163, 184, 150)
    };
    let corner_idle = if dark_mode {
        Color32::from_rgb(30, 41, 59)
    } else {
        Color32::from_rgb(226, 232, 240)
    };
    let corner_fg = if dark_mode {
        Color32::WHITE
    } else {
        Color32::from_rgb(15, 23, 42)
    };
    let face_idle = if dark_mode {
        Color32::from_rgba_unmultiplied(30, 41, 59, 240)
    } else {
        Color32::from_rgba_unmultiplied(241, 245, 249, 245)
    };

    egui::Area::new(egui::Id::new("hud_3d_viewcube"))
        .fixed_pos(viewport_rect.min + egui::vec2(14.0, 50.0))
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(vc_bg)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(8))
                .stroke(Stroke::new(1.0_f32, vc_border))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        let stage_size = egui::vec2(96.0, 90.0);
                        let (stage_rect, stage_resp) =
                            ui.allocate_exact_size(stage_size, Sense::click_and_drag());
                        if stage_resp.dragged_by(egui::PointerButton::Primary) {
                            let d = stage_resp.drag_delta();
                            camera.yaw -= d.x * 0.015;
                            camera.pitch = (camera.pitch + d.y * 0.015).clamp(-1.48, 1.48);
                        }

                        let painter = ui.painter_at(stage_rect);
                        let center = stage_rect.center();

                        // 4 Isometric corner buttons around the cube
                        let mut clicked_corner: Option<CameraViewPreset> = None;
                        let corners = [
                            (stage_rect.min + egui::vec2(10.0, 10.0), "NW", CameraViewPreset::IsoNw),
                            (Pos2::new(stage_rect.max.x - 10.0, stage_rect.min.y + 10.0), "NE", CameraViewPreset::IsoNe),
                            (Pos2::new(stage_rect.min.x + 10.0, stage_rect.max.y - 10.0), "SW", CameraViewPreset::IsoSw),
                            (stage_rect.max - egui::vec2(10.0, 10.0), "SE", CameraViewPreset::IsoSe),
                        ];
                        for (cpos, clbl, preset) in corners {
                            let crect = Rect::from_center_size(cpos, egui::vec2(19.0, 15.0));
                            let hov = ui.input(|i| i.pointer.hover_pos().map_or(false, |hp| crect.contains(hp)));
                            painter.rect_filled(
                                crect,
                                3.0,
                                if hov { Color32::from_rgb(2, 132, 199) } else { corner_idle },
                            );
                            painter.text(
                                crect.center(),
                                egui::Align2::CENTER_CENTER,
                                clbl,
                                FontId::proportional(8.5),
                                if hov { Color32::WHITE } else { corner_fg },
                            );
                            if hov && stage_resp.clicked() {
                                clicked_corner = Some(preset);
                            }
                        }

                        // Project unit cube rotated to match camera yaw & pitch
                        let cy = (-camera.yaw).cos();
                        let sy = (-camera.yaw).sin();
                        let cp = camera.pitch.cos();
                        let sp = camera.pitch.sin();
                        let project = |p: [f32; 3]| -> (Pos2, f32) {
                            let x1 = p[0] * cy - p[2] * sy;
                            let z1 = p[0] * sy + p[2] * cy;
                            let y2 = p[1] * cp - z1 * sp;
                            let z2 = p[1] * sp + z1 * cp;
                            let scale = 25.0 * (4.2 / (4.2 - z2 * 0.45));
                            (Pos2::new(center.x + x1 * scale, center.y - y2 * scale), z2)
                        };
                        let xform_norm = |n: [f32; 3]| -> [f32; 3] {
                            let x1 = n[0] * cy - n[2] * sy;
                            let z1 = n[0] * sy + n[2] * cy;
                            let y2 = n[1] * cp - z1 * sp;
                            let z2 = n[1] * sp + z1 * cp;
                            [x1, y2, z2]
                        };

                        let cube_verts: [[f32; 3]; 8] = [
                            [-1.0, -1.0, -1.0],
                            [ 1.0, -1.0, -1.0],
                            [ 1.0,  1.0, -1.0],
                            [-1.0,  1.0, -1.0],
                            [-1.0, -1.0,  1.0],
                            [ 1.0, -1.0,  1.0],
                            [ 1.0,  1.0,  1.0],
                            [-1.0,  1.0,  1.0],
                        ];
                        let faces: [([usize; 4], [f32; 3], &str, CameraViewPreset, Color32); 6] = [
                            ([4, 5, 6, 7], [ 0.0,  0.0,  1.0], if is_de { "VOR" } else { "FRONT" }, CameraViewPreset::Front, Color32::from_rgb(59, 130, 246)),
                            ([1, 0, 3, 2], [ 0.0,  0.0, -1.0], if is_de { "HIN" } else { "BACK" },  CameraViewPreset::Back,  Color32::from_rgb(59, 130, 246)),
                            ([5, 1, 2, 6], [ 1.0,  0.0,  0.0], if is_de { "REC" } else { "RIGHT" }, CameraViewPreset::Right, Color32::from_rgb(239, 68, 68)),
                            ([0, 4, 7, 3], [-1.0,  0.0,  0.0], if is_de { "LIN" } else { "LEFT" },  CameraViewPreset::Left,  Color32::from_rgb(239, 68, 68)),
                            ([3, 2, 6, 7], [ 0.0,  1.0,  0.0], if is_de { "OBN" } else { "TOP" },   CameraViewPreset::Top,   Color32::from_rgb(34, 197, 94)),
                            ([0, 1, 5, 4], [ 0.0, -1.0,  0.0], if is_de { "UNT" } else { "BOT" },   CameraViewPreset::Bottom,Color32::from_rgb(34, 197, 94)),
                        ];

                        let mut vis: Vec<(f32, Vec<Pos2>, &str, CameraViewPreset, Color32)> = faces
                            .iter()
                            .filter_map(|(idx, n, lbl, preset, col)| {
                                let vn = xform_norm(*n);
                                if vn[2] <= 0.05 {
                                    return None;
                                }
                                let mut spts = Vec::with_capacity(4);
                                let mut z_sum = 0.0_f32;
                                for &vi in idx {
                                    let (sp_v, zv) = project(cube_verts[vi]);
                                    spts.push(sp_v);
                                    z_sum += zv;
                                }
                                Some((z_sum * 0.25, spts, *lbl, *preset, *col))
                            })
                            .collect();
                        vis.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

                        let hover_pos = ui.input(|i| i.pointer.hover_pos());
                        let mut clicked_face: Option<CameraViewPreset> = None;
                        for (_, spts, lbl, preset, accent) in &vis {
                            let frect = Rect::from_points(spts);
                            let hov = clicked_corner.is_none()
                                && hover_pos.map_or(false, |hp| frect.contains(hp));
                            let fill = if hov {
                                Color32::from_rgb(2, 132, 199)
                            } else {
                                face_idle
                            };
                            painter.add(egui::Shape::convex_polygon(
                                spts.clone(),
                                fill,
                                Stroke::new(1.3_f32, *accent),
                            ));
                            let c_face = Pos2::new(
                                spts.iter().map(|p| p.x).sum::<f32>() * 0.25,
                                spts.iter().map(|p| p.y).sum::<f32>() * 0.25,
                            );
                            painter.text(
                                c_face,
                                egui::Align2::CENTER_CENTER,
                                *lbl,
                                FontId::proportional(9.5),
                                if hov { Color32::WHITE } else { corner_fg },
                            );
                            if hov && stage_resp.clicked() {
                                clicked_face = Some(*preset);
                            }
                        }
                        if let Some(p) = clicked_corner.or(clicked_face) {
                            camera.apply_preset(p, layout);
                        }

                        ui.add_space(3.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            if ui.small_button("↶45°").on_hover_text(if is_de { "45° nach links" } else { "Rotate 45° left" }).clicked() {
                                camera.yaw -= std::f32::consts::FRAC_PI_4;
                            }
                            if ui.small_button("⌂ ISO").on_hover_text(if is_de { "Standard 3D-Ansicht" } else { "Home 3D Isometric View" }).clicked() {
                                camera.apply_preset(CameraViewPreset::Home, layout);
                            }
                            if ui.small_button("↷45°").on_hover_text(if is_de { "45° nach rechts" } else { "Rotate 45° right" }).clicked() {
                                camera.yaw += std::f32::consts::FRAC_PI_4;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            if ui.small_button(if is_de { "Oben" } else { "Top" }).clicked() {
                                camera.apply_preset(CameraViewPreset::Top, layout);
                            }
                            if ui.small_button(if is_de { "Vor" } else { "Front" }).clicked() {
                                camera.apply_preset(CameraViewPreset::Front, layout);
                            }
                            if ui.small_button(if is_de { "Rechts" } else { "Right" }).clicked() {
                                camera.apply_preset(CameraViewPreset::Right, layout);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            if ui.small_button(if is_de { "Links" } else { "Left" }).clicked() {
                                camera.apply_preset(CameraViewPreset::Left, layout);
                            }
                            if ui.small_button(if is_de { "Hinten" } else { "Back" }).clicked() {
                                camera.apply_preset(CameraViewPreset::Back, layout);
                            }
                            if ui.small_button(if is_de { "Unten" } else { "Bot" }).clicked() {
                                camera.apply_preset(CameraViewPreset::Bottom, layout);
                            }
                        });
                    });
                });
        });
}

// ============================================================================
//  Batched 3D Scene Mesh Builder (Baseboard + Detailed Tracks + Models + Terrain)
// ============================================================================

// (Corridor chains built cleanly via terrain_tools::build_connected_corridor_chains)

pub(super) fn board_style_rgb(style: &str) -> ([f32; 3], [f32; 3]) {
    match style {
        "grass" => ([0.22, 0.42, 0.14], [0.16, 0.30, 0.10]),
        "dark-wood" => ([0.34, 0.22, 0.14], [0.24, 0.15, 0.09]),
        "plywood" => ([0.82, 0.69, 0.48], [0.64, 0.50, 0.33]),
        "slate" => ([0.22, 0.28, 0.36], [0.16, 0.20, 0.27]),
        "white" => ([0.88, 0.90, 0.92], [0.74, 0.77, 0.80]),
        _ => ([0.68, 0.54, 0.36], [0.46, 0.34, 0.21]),
    }
}

