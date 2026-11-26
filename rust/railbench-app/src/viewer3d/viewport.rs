use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn build_uniforms_3d(
    view_proj: [[f32; 4]; 4],
    cached_bundle: &Option<Scene3DBundle>,
    sky: &SkyUniformState,
    eye: [f32; 3],
    cam_right: [f32; 3],
    cam_up: [f32; 3],
    cam_fwd: [f32; 3],
    aspect: f32,
    tan_half_y: f32,
    env_mode: Viewer3DEnv,
    low_gpu_mode: bool,
) -> Uniforms3D {
    let (shadow_center, shadow_half_extent) = cached_bundle
        .as_ref()
        .map(|b| (b.shadow_center, b.shadow_half_extent))
        .unwrap_or(([0.0, 40.0, 0.0], 1600.0));
    let sun_view_proj = compute_sun_view_proj(
        [sky.sun_dir[0], sky.sun_dir[1], sky.sun_dir[2]],
        shadow_center,
        shadow_half_extent,
    );
    let night = sky.hemi_ground_pad[3];
    let shadow_strength = if low_gpu_mode {
        0.0
    } else {
        let base = if env_mode == Viewer3DEnv::Minimal { 0.45 } else { 0.62 };
        base * (1.0 - 0.55 * night)
    };
    let normal_bias = (shadow_half_extent / SHADOW_MAP_SIZE as f32 * 3.5).clamp(2.5, 12.0);
    let is_minimal_f = if env_mode == Viewer3DEnv::Minimal { 1.0 } else { 0.0 };
    Uniforms3D {
        view_proj,
        sun_view_proj,
        sun_dir: sky.sun_dir,
        cam_pos: [eye[0], eye[1], eye[2], 1.0],
        fog_color: sky.fog_color,
        sun_color_int: sky.sun_color_int,
        hemi_sky_int: sky.hemi_sky_int,
        hemi_ground_pad: sky.hemi_ground_pad,
        cam_right: [cam_right[0], cam_right[1], cam_right[2], aspect],
        cam_up: [cam_up[0], cam_up[1], cam_up[2], tan_half_y],
        cam_fwd: [cam_fwd[0], cam_fwd[1], cam_fwd[2], is_minimal_f],
        sky_zenith: sky.sky_zenith,
        shadow_params: [shadow_strength, 1.0 / SHADOW_MAP_SIZE as f32, normal_bias, 0.0005],
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_viewport_3d(
    ui: &mut egui::Ui,
    camera: &mut Camera3D,
    cached_bundle: &Option<Scene3DBundle>,
    vehicles_mesh: Option<Arc<SceneMesh3D>>,
    edit: Edit3DParams<'_>,
    env_mode: &mut Viewer3DEnv,
    day_night_active: &mut bool,
    time_of_day: &mut f32,
    day_night_speed: &mut f32,
    show_wood_plates: &mut bool,
    first_short_circuit: Option<Vec2>,
    dark_mode: bool,
    low_gpu_mode: bool,
    lang: Lang,
    dt_secs: f32,
) -> Edit3DOutput {
    if *day_night_active {
        let delta_hours = dt_secs * (24.0 / 1200.0) * (*day_night_speed);
        *time_of_day = (*time_of_day + delta_hours).rem_euclid(24.0);
    }

    let avail = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());

    let sculpt_mode = edit.terrain_brush.active && edit.terrain_brush.is_heightfield_mode();
    let (consumed_primary_drag, consumed_scroll, edit_out) = if sculpt_mode {
        handle_3d_terrain_sculpt_interaction(
            ui,
            rect,
            &response,
            camera,
            edit.terrain_brush,
            edit.layout,
        )
    } else {
        let (cd, eo) = handle_3d_edit_interaction(
            ui,
            rect,
            &response,
            camera,
            edit.state,
            edit.layout,
            edit.catalogs,
            edit.selected,
            edit.dragging_catalog,
            edit.dragging_model,
            edit.snapping_enabled,
            edit.layer_protection,
            edit.rotation_step,
            *env_mode,
            edit.terrain_only_selection,
        );
        (cd, false, eo)
    };

    let shift_held = ui.input(|i| i.modifiers.shift);
    // Full 360° / 180° Orbit (left drag on empty space, or right-drag while sculpting terrain)
    if (!consumed_primary_drag && response.dragged_by(egui::PointerButton::Primary))
        || (sculpt_mode && !shift_held && response.dragged_by(egui::PointerButton::Secondary))
    {
        let d = response.drag_delta();
        camera.yaw -= d.x * 0.0065;
        camera.pitch = (camera.pitch + d.y * 0.0065).clamp(-1.48, 1.48);
    }
    // Pan (right or middle drag; Shift+right or middle drag while sculpting)
    if response.dragged_by(egui::PointerButton::Middle)
        || ((!sculpt_mode || shift_held) && response.dragged_by(egui::PointerButton::Secondary))
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
    if !consumed_scroll && response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.01 {
            let factor = (1.0 - scroll * 0.0018).clamp(0.75, 1.35);
            camera.distance = (camera.distance * factor).clamp(60.0, 25000.0);
        }
    }

    let ppp = ui.ctx().pixels_per_point();
    let res_scale = if low_gpu_mode { 0.75_f32 } else { 1.0_f32 };
    let px_w = ((rect.width() * ppp * res_scale).round() as u32).max(16);
    let px_h = ((rect.height() * ppp * res_scale).round() as u32).max(16);
    let aspect = (rect.width() / rect.height().max(1.0)).max(0.1);
    let eye = camera.eye();
    let sky = eval_sky_state(*time_of_day, *env_mode, dark_mode);
    let (cam_right, cam_up, cam_fwd) = camera_basis(eye, camera.target);
    let tan_half_y = (45.0_f32.to_radians() * 0.5).tan();
    let uniforms = build_uniforms_3d(
        camera.view_proj(aspect),
        cached_bundle,
        &sky,
        eye,
        cam_right,
        cam_up,
        cam_fwd,
        aspect,
        tan_half_y,
        *env_mode,
        low_gpu_mode,
    );

    let cb = egui_wgpu::Callback::new_paint_callback(
        rect,
        Viewport3DCallback {
            uniforms,
            bundle: cached_bundle.clone(),
            vehicles_mesh,
            time_of_day: *time_of_day,
            env_mode: *env_mode,
            viewport_px: [px_w, px_h],
            clear_color: sky.clear_color,
            is_cab_view: false,
            shadows_enabled: !low_gpu_mode,
            day_night_active: *day_night_active,
        },
    );
    ui.painter().add(cb);

    let is_de = lang == Lang::De;
    let p = crate::theme::palette(dark_mode);

    // Top-left 3D Controls Bar (Hall & Landscape | Minimal CAD, Wood Plates, Edit 3D, Day/Night)
    let top_bar_resp = egui::Area::new(egui::Id::new("hud_3d_top_bar"))
        .fixed_pos(rect.min + egui::vec2(14.0, 12.0))
        .show(ui.ctx(), |ui| {
            hud_3d_panel_frame(dark_mode, None).show(ui, |ui| {
                ui.spacing_mut().interact_size.y = HUD_3D_BTN_H;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    // 3D Icon + Title badge
                    Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(
                            p.primary.r(),
                            p.primary.g(),
                            p.primary.b(),
                            if dark_mode { 36 } else { 26 },
                        ))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(7, 3))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                let (icon_r, _) =
                                    ui.allocate_exact_size(egui::vec2(13.0, 13.0), Sense::hover());
                                crate::icons::paint_icon(
                                    ui.painter(),
                                    icon_r,
                                    crate::icons::Icon::Box3d,
                                    p.primary,
                                );
                                ui.label(RichText::new("3D").strong().size(11.5).color(p.primary));
                            });
                        });

                    // Segmented Environment Switcher (Hall & Landscape | Minimal CAD)
                    Frame::new()
                        .fill(p.bg_input)
                        .corner_radius(CornerRadius::same(7))
                        .inner_margin(Margin::same(2))
                        .stroke(Stroke::new(1.0_f32, p.border_main))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            ui.horizontal(|ui| {
                                let hall_lbl =
                                    if is_de { "Halle & Landschaft" } else { "Hall & Landscape" };
                                let hall_active = *env_mode == Viewer3DEnv::HallLandscape;
                                let hall_w = crate::ui_panels::stable_text_width(
                                    ui,
                                    "Halle & Landschaft",
                                    "Hall & Landscape",
                                    Some(11.5),
                                    true,
                                ) + 16.0;
                                let mut hall_rt = RichText::new(hall_lbl).size(11.5).color(if hall_active {
                                    Color32::WHITE
                                } else {
                                    p.text_sub
                                });
                                if hall_active {
                                    hall_rt = hall_rt.strong();
                                }
                                if ui
                                    .add(
                                        egui::Button::new(hall_rt)
                                            .fill(if hall_active {
                                                p.primary_hover
                                            } else {
                                                Color32::TRANSPARENT
                                            })
                                            .stroke(Stroke::NONE)
                                            .min_size(egui::vec2(hall_w, 22.0))
                                            .corner_radius(CornerRadius::same(5)),
                                    )
                                    .on_hover_text(if is_de {
                                        "Historische Ausstellungshalle mit umliegender Gebirgslandschaft"
                                    } else {
                                        "Exhibition Hall with horizon-spanning Alpine landscape"
                                    })
                                    .clicked()
                                {
                                    *env_mode = Viewer3DEnv::HallLandscape;
                                }

                                let min_active = *env_mode == Viewer3DEnv::Minimal;
                                let min_w = crate::ui_panels::stable_text_width(
                                    ui,
                                    "Minimal CAD",
                                    "Minimal CAD",
                                    Some(11.5),
                                    true,
                                ) + 16.0;
                                let mut min_rt = RichText::new("Minimal CAD").size(11.5).color(if min_active {
                                    Color32::WHITE
                                } else {
                                    p.text_sub
                                });
                                if min_active {
                                    min_rt = min_rt.strong();
                                }
                                if ui
                                    .add(
                                        egui::Button::new(min_rt)
                                            .fill(if min_active {
                                                p.primary_hover
                                            } else {
                                                Color32::TRANSPARENT
                                            })
                                            .stroke(Stroke::NONE)
                                            .min_size(egui::vec2(min_w, 22.0))
                                            .corner_radius(CornerRadius::same(5)),
                                    )
                                    .on_hover_text(if is_de {
                                        "Minimaler CAD-Modus mit direktem Gleisbau"
                                    } else {
                                        "Minimal CAD view with direct track building"
                                    })
                                    .clicked()
                                {
                                    *env_mode = Viewer3DEnv::Minimal;
                                    edit.state.enabled = true;
                                }
                            });
                        });

                    ui.separator();
                    let wood_lbl = if is_de { "Holzplatten" } else { "Wood Plates" };
                    if hud_3d_stable_toggle(
                        ui,
                        dark_mode,
                        *show_wood_plates,
                        wood_lbl,
                        "Holzplatten",
                        "Wood Plates",
                    )
                    .clicked()
                    {
                        *show_wood_plates = !*show_wood_plates;
                    }

                    let edit_lbl = if is_de { "3D Bearbeiten" } else { "Edit 3D" };
                    if hud_3d_stable_toggle(
                        ui,
                        dark_mode,
                        edit.state.enabled || *env_mode == Viewer3DEnv::Minimal,
                        edit_lbl,
                        "3D Bearbeiten",
                        "Edit 3D",
                    )
                    .on_hover_text(if is_de {
                        "Direkte 3D-Bearbeitung & Gleisbau ein/ausschalten"
                    } else {
                        "Toggle direct 3D track adding and editing"
                    })
                    .clicked()
                    {
                        edit.state.enabled = !edit.state.enabled;
                    }

                    if *env_mode != Viewer3DEnv::Minimal {
                        ui.separator();
                        let h = (*time_of_day).floor() as u32 % 24;
                        let m = (((*time_of_day) - (*time_of_day).floor()) * 60.0).floor() as u32 % 60;
                        let play_sym = if *day_night_active { "||" } else { ">" };
                        if hud_3d_pill_button(
                            ui,
                            dark_mode,
                            &format!("{play_sym} {h:02}:{m:02}"),
                            *day_night_active,
                            true,
                            56.0,
                        )
                        .clicked()
                        {
                            *day_night_active = !*day_night_active;
                        }
                        ui.add_sized(
                            [86.0, 18.0],
                            egui::Slider::new(time_of_day, 0.0..=24.0).show_value(false),
                        );
                        if hud_3d_pill_button(
                            ui,
                            dark_mode,
                            &format!("{}x", *day_night_speed as u32),
                            false,
                            true,
                            36.0,
                        )
                        .clicked()
                        {
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
                    }

                    if let Some(sc) = first_short_circuit {
                        ui.separator();
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("SHORT!").strong().size(11.0).color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(220, 38, 38))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(248, 113, 113)))
                                .min_size(egui::vec2(58.0, HUD_3D_BTN_H))
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .on_hover_text("Focus 3D camera on short circuit point")
                            .clicked()
                        {
                            camera.focus_point(sc.x, sc.y);
                        }
                    }

                    ui.separator();
                    let hint_txt = if sculpt_mode {
                        if is_de {
                            "Ziehen: Formen • Shift: Glaetten • Alt: Invert • Shift/Ctrl+Rad: R/Staerke • RMB: Orbit"
                        } else {
                            "Drag: Sculpt • Shift: Smooth • Alt: Invert • Shift/Ctrl+Wheel: R/Strength • RMB: Orbit"
                        }
                    } else if edit.state.enabled || *env_mode == Viewer3DEnv::Minimal {
                        if is_de {
                            "Ziehen/Klick '+' : Gleis bauen • Klick: Auswaehlen • Alt+Ziehen: Hoehe • Leer: Orbit"
                        } else {
                            "Drag/Click '+': Add Track • Click: Select • Alt+Drag: Height • Empty: Orbit"
                        }
                    } else {
                        "LMB: Orbit  •  RMB: Pan  •  Wheel: Zoom"
                    };
                    ui.label(RichText::new(hint_txt).size(10.5).color(p.text_muted));
                });
            });
        });

    // Interactive CAD ViewCube Widget anchored cleanly below the top HUD bar
    let viewcube_top_y = (top_bar_resp.response.rect.bottom() + 8.0).max(rect.min.y + 56.0);
    super::viewcube::draw_viewcube_widget(ui, rect, viewcube_top_y, camera, edit.layout, is_de, dark_mode);

    if sculpt_mode {
        draw_3d_terrain_brush_overlay(
            ui,
            rect,
            camera,
            edit.terrain_brush,
            edit.layout,
            dark_mode,
            lang,
        );
    } else {
        // 3D Edit Mode selection wireframes, drop ghosts & rotation handle (bottom bar is unified via draw_floating_selection_bar)
        draw_3d_edit_overlays(
            ui,
            rect,
            camera,
            edit.state,
            edit.layout,
            edit.catalogs,
            edit.selected,
            edit.dragging_catalog,
            edit.dragging_model,
            *env_mode,
        );
    }

    edit_out
}

pub(super) const HUD_3D_BTN_H: f32 = 24.0;

pub(super) fn hud_3d_panel_frame(dark_mode: bool, accent_border: Option<Color32>) -> Frame {
    let fill = if dark_mode {
        Color32::from_rgba_unmultiplied(15, 23, 42, 232)
    } else {
        Color32::from_rgba_unmultiplied(245, 240, 230, 242)
    };
    let border = accent_border.unwrap_or_else(|| {
        if dark_mode {
            Color32::from_rgba_unmultiplied(148, 163, 184, 72)
        } else {
            Color32::from_rgba_unmultiplied(180, 170, 154, 195)
        }
    });
    Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(10, 6))
        .stroke(Stroke::new(1.0_f32, border))
        .shadow(egui::Shadow {
            offset: [0, 5],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(if dark_mode { 110 } else { 42 }),
        })
}

pub(super) fn hud_3d_pill_button(
    ui: &mut egui::Ui,
    dark_mode: bool,
    label: &str,
    active: bool,
    enabled: bool,
    min_w: f32,
) -> egui::Response {
    let p = crate::theme::palette(dark_mode);
    let (fill, border_col, txt_col) = if !enabled {
        (
            p.bg_sidebar,
            Color32::from_rgba_unmultiplied(p.border_main.r(), p.border_main.g(), p.border_main.b(), 140),
            p.text_muted,
        )
    } else if active {
        (p.primary_hover, p.primary, Color32::WHITE)
    } else {
        (p.bg_card, p.border_main, p.text_main)
    };
    let mut rt = RichText::new(label).size(11.5).color(txt_col);
    if active {
        rt = rt.strong();
    }
    ui.add_enabled(
        enabled,
        egui::Button::new(rt)
            .fill(fill)
            .stroke(Stroke::new(1.0_f32, border_col))
            .min_size(egui::vec2(min_w, HUD_3D_BTN_H))
            .corner_radius(CornerRadius::same(6)),
    )
}

pub(super) fn hud_3d_stable_toggle(
    ui: &mut egui::Ui,
    dark_mode: bool,
    active: bool,
    active_lbl: &str,
    de_lbl: &str,
    en_lbl: &str,
) -> egui::Response {
    let w = crate::ui_panels::stable_text_width(ui, de_lbl, en_lbl, Some(11.5), true) + 18.0;
    hud_3d_pill_button(ui, dark_mode, active_lbl, active, true, w)
}


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

