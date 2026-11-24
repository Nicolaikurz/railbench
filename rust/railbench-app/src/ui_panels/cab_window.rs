use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Stroke};

use crate::i18n::Lang;
use crate::vehicle_sim::{VehicleDomain, VehicleKind};
use crate::viewer3d::{draw_cab_viewport_3d, CabCameraMode, SceneMesh3D, Viewer3DEnv};
use crate::RailbenchApp;

/// Renders a dedicated, independent OS window (`ViewportId`) containing:
/// - Left/Center: Live 3D First-Person Cab View (or Roof/Chase camera) looking out from the active locomotive
/// - Right Panel: Full Locomotive & Turnout Controller so the user can place this window on a second monitor.
pub fn draw_cab_view_window(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let title = if is_de {
        "Railbench — Fahrbetrieb, Fuehrerstand & Stellwerk (2. Monitor)"
    } else {
        "Railbench — Drive, Cab View & Signal Box (2nd Monitor)"
    };

    let vp_id = egui::ViewportId::from_hash_of("railbench_cab_view_window");
    let builder = egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size([1220.0, 760.0])
        .with_min_inner_size([680.0, 440.0]);

    // Ensure 3D scene & vehicle mesh are up to date even when the main window is in 2D mode
    app.ensure_mesh_3d();
    app.gpu_3d_resident = true;
    let vehicles_mesh = if !app.vehicle_sim.vehicles.is_empty() {
        let (verts, idxs) = app.vehicle_sim.build_vehicles_mesh_3d(&app.layout.scale);
        if !idxs.is_empty() {
            Some(std::sync::Arc::new(SceneMesh3D::from_raw(verts, idxs)))
        } else {
            None
        }
    } else {
        None
    };

    ctx.show_viewport_immediate(vp_id, builder, |vp_ctx, _class| {
        if vp_ctx.input(|i| i.viewport().close_requested()) {
            app.show_cab_window = false;
            return;
        }
        let p = crate::theme::palette(app.dark_mode);

        // Pick active vehicle (prefer selected_vehicle_id, else first rail loco, else first vehicle)
        let active_idx = app
            .vehicle_sim
            .selected_vehicle_id
            .as_deref()
            .and_then(|sid| app.vehicle_sim.vehicles.iter().position(|v| v.id == sid))
            .or_else(|| app.vehicle_sim.vehicles.iter().position(|v| v.kind.is_locomotive()))
            .or_else(|| if !app.vehicle_sim.vehicles.is_empty() { Some(0) } else { None });

        let mut spawn_req: Option<VehicleKind> = None;
        let mut remove_id: Option<String> = None;
        let mut focus_pt: Option<(f64, f64)> = None;
        let mut add_wagon_req: Option<(String, VehicleKind)> = None;
        let mut rem_wagon_id: Option<String> = None;
        let mut toggle_turnout_req: Option<(String, usize)> = None;

        // Top View Mode Switcher inside the unified Drive & Cab Window
        egui::TopBottomPanel::top("cab_window_mode_bar")
            .frame(
                Frame::new()
                    .fill(p.bg_toolbar)
                    .inner_margin(Margin::symmetric(10, 5))
                    .stroke(Stroke::new(1.0_f32, p.border_main)),
            )
            .show(vp_ctx, |ui| {
                ui.horizontal(|ui| {
                    for (mode_id, lbl_de, lbl_en) in [
                        (2u8, "◫ Geteilt (Cab + Stellwerk)", "◫ Split (Cab + Signal Box)"),
                        (0u8, "🎥 Fuehrerstand", "🎥 Cab View"),
                        (1u8, "🎛 Stellwerk", "🎛 Signal Box"),
                    ] {
                        let active = app.cab_view_layout == mode_id;
                        if ui
                            .selectable_label(active, RichText::new(if is_de { lbl_de } else { lbl_en }).strong().size(11.5))
                            .clicked()
                        {
                            app.cab_view_layout = mode_id;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if app.cab_view_layout >= 1
                            && ui.small_button(if is_de { "⌖ Einpassen" } else { "⌖ Fit" }).clicked()
                        {
                            app.stellwerk_zoom = 1.0;
                            app.stellwerk_pan = [0.0, 0.0];
                            app.stellwerk_layer_views.clear();
                        }
                    });
                });
            });

        // Right Docked Control Desk (Führerstand-Pult)
        egui::SidePanel::right("cab_control_desk")
            .exact_width(292.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(p.bg_sidebar)
                    .inner_margin(Margin::same(10))
                    .stroke(Stroke::new(1.0_f32, p.border_main)),
            )
            .show(vp_ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(if is_de { "🚂 Fuehrerstand-Pult" } else { "🚂 Cab Control Desk" })
                                .strong()
                                .size(14.0),
                        );
                    });
                    ui.add_space(4.0);

                    // Master Play/Pause + Emergency Stop
                    ui.horizontal(|ui| {
                        let play_lbl = if app.vehicle_sim.running {
                            if is_de { "⏸ Pause" } else { "⏸ Pause" }
                        } else if is_de {
                            "▶ Fahren"
                        } else {
                            "▶ Start"
                        };
                        let play_btn = egui::Button::new(RichText::new(play_lbl).strong().color(Color32::WHITE))
                            .fill(if app.vehicle_sim.running {
                                Color32::from_rgb(217, 119, 6)
                            } else {
                                Color32::from_rgb(22, 163, 74)
                            });
                        if ui.add(play_btn).clicked() {
                            app.vehicle_sim.running = !app.vehicle_sim.running;
                        }
                        let stop_btn = egui::Button::new(
                            RichText::new(if is_de { "⏹ Not-Halt" } else { "⏹ E-Stop" })
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(185, 28, 28));
                        if ui.add(stop_btn).clicked() {
                            app.vehicle_sim.emergency_stop_all();
                        }
                        if !app.vehicle_sim.vehicles.is_empty()
                            && ui.small_button(if is_de { "Alle ✕" } else { "Clear" }).clicked()
                        {
                            app.vehicle_sim.clear_all();
                        }
                    });

                    ui.add_space(6.0);
                    // Camera View Mode Selector
                    ui.label(RichText::new(if is_de { "Kameraposition" } else { "Camera View" }).strong().size(11.5));
                    ui.horizontal(|ui| {
                        for (mode, lbl_de, lbl_en) in [
                            (CabCameraMode::Cab, "Fuehrerstand", "Cab"),
                            (CabCameraMode::Roof, "Dach", "Roof"),
                            (CabCameraMode::Chase, "Verfolger", "Chase"),
                        ] {
                            let active = app.cab_camera_mode == mode;
                            if ui
                                .selectable_label(active, if is_de { lbl_de } else { lbl_en })
                                .clicked()
                            {
                                app.cab_camera_mode = mode;
                                app.cab_look_yaw = 0.0;
                                app.cab_look_pitch = 0.0;
                            }
                        }
                        if ui.small_button("⌖").on_hover_text(if is_de { "Blick zentrieren" } else { "Center look" }).clicked() {
                            app.cab_look_yaw = 0.0;
                            app.cab_look_pitch = 0.0;
                        }
                    });

                    // Quick Environment / Time of Day controls
                    ui.horizontal(|ui| {
                        let env_lbl = match app.viewer3d_env {
                            Viewer3DEnv::HallLandscape => if is_de { "Halle" } else { "Hall" },
                            Viewer3DEnv::Minimal => "CAD",
                        };
                        if ui.small_button(env_lbl).clicked() {
                            app.viewer3d_env = match app.viewer3d_env {
                                Viewer3DEnv::HallLandscape => Viewer3DEnv::Minimal,
                                Viewer3DEnv::Minimal => Viewer3DEnv::HallLandscape,
                            };
                        }
                        let dn_lbl = if app.time_of_day >= 19.5 || app.time_of_day < 6.0 {
                            if is_de { "🌙 Nacht" } else { "🌙 Night" }
                        } else if is_de {
                            "☀ Tag"
                        } else {
                            "☀ Day"
                        };
                        if ui.small_button(dn_lbl).clicked() {
                            app.time_of_day = if app.time_of_day >= 19.5 || app.time_of_day < 6.0 {
                                13.5
                            } else {
                                22.0
                            };
                        }
                    });

                    ui.separator();

                    // Active Vehicle Selector (if multiple vehicles)
                    if !app.vehicle_sim.vehicles.is_empty() {
                        ui.label(RichText::new(if is_de { "Aktives Fahrzeug" } else { "Active Vehicle" }).strong().size(11.5));
                        ui.horizontal_wrapped(|ui| {
                            for (i, v) in app.vehicle_sim.vehicles.iter().enumerate() {
                                let is_cur = active_idx == Some(i);
                                if ui.selectable_label(is_cur, &v.name).clicked() {
                                    app.vehicle_sim.selected_vehicle_id = Some(v.id.clone());
                                    app.cab_look_yaw = 0.0;
                                    app.cab_look_pitch = 0.0;
                                }
                            }
                        });
                        ui.add_space(4.0);
                    }

                    // Active Locomotive Controls
                    if let Some(idx) = active_idx {
                        let scale_str = app.layout.scale.clone();
                        let v = &mut app.vehicle_sim.vehicles[idx];
                        let kmh = v.scale_speed_kmh(&scale_str);
                        let tons = v.total_mass_tons();

                        Frame::new()
                            .fill(p.bg_card)
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(8))
                            .stroke(Stroke::new(1.0_f32, p.border_main))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&v.name).strong().size(13.0));
                                    if v.blocked_ahead {
                                        ui.label(
                                            RichText::new(if is_de { "⚠ Blockiert" } else { "⚠ Blocked" })
                                                .size(11.0)
                                                .strong()
                                                .color(Color32::from_rgb(245, 158, 11)),
                                        );
                                    }
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.small_button("✕").clicked() {
                                            remove_id = Some(v.id.clone());
                                        }
                                        if ui.small_button("⌖").on_hover_text(if is_de { "Auf 2D fokussieren" } else { "Focus on 2D" }).clicked() {
                                            focus_pt = Some((v.world_x, v.world_y));
                                        }
                                    });
                                });
                                ui.label(
                                    RichText::new(format!(
                                        "{kmh:.0} km/h  |  {tons:.0} t  |  {:+.1}%",
                                        v.grade_pct
                                    ))
                                    .size(11.5)
                                    .color(p.text_muted),
                                );
                                ui.add_space(4.0);

                                // Big Throttle Slider
                                ui.label(RichText::new(if is_de { "Fahrregler" } else { "Throttle" }).small());
                                ui.add(
                                    egui::Slider::new(&mut v.throttle, 0.0..=1.0)
                                        .custom_formatter(|val, _| format!("{:.0}%", val * 100.0)),
                                );
                                // Quick Notch Buttons
                                ui.horizontal(|ui| {
                                    for &(pct, val) in &[("0%", 0.0_f32), ("25%", 0.25), ("50%", 0.5), ("75%", 0.75), ("100%", 1.0)] {
                                        if ui.small_button(pct).clicked() {
                                            v.throttle = val;
                                            if val > 0.0 {
                                                app.vehicle_sim.running = true;
                                            }
                                        }
                                    }
                                });
                                ui.add_space(4.0);

                                // Direction & Headlights
                                ui.horizontal(|ui| {
                                    let dir_lbl = if v.reverse {
                                        if is_de { "◂ Rueckwaerts" } else { "◂ Reverse" }
                                    } else if is_de {
                                        "▸ Vorwaerts"
                                    } else {
                                        "▸ Forward"
                                    };
                                    if ui.button(dir_lbl).clicked() {
                                        v.reverse = !v.reverse;
                                    }
                                    let light_btn = egui::Button::new(if v.headlights { "💡 ON" } else { "💡 OFF" })
                                        .fill(if v.headlights {
                                            Color32::from_rgb(202, 138, 4)
                                        } else {
                                            p.bg_input
                                        });
                                    if ui.add(light_btn).clicked() {
                                        v.headlights = !v.headlights;
                                    }
                                });

                                // Consist Wagons
                                if v.kind.is_locomotive() {
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{}: {}",
                                                if is_de { "Wagen" } else { "Wagons" },
                                                v.coupled_cars.len()
                                            ))
                                            .small(),
                                        );
                                        if ui.small_button("+W").clicked() {
                                            add_wagon_req = Some((v.id.clone(), VehicleKind::PassengerCoach));
                                        }
                                        if ui.small_button("+G").clicked() {
                                            add_wagon_req = Some((v.id.clone(), VehicleKind::FreightCar));
                                        }
                                        if !v.coupled_cars.is_empty() && ui.small_button("-W").clicked() {
                                            rem_wagon_id = Some(v.id.clone());
                                        }
                                    });
                                }
                            });
                    } else {
                        ui.label(
                            RichText::new(if is_de {
                                "Keine Lok aktiv. Platziere unten eine Lok:"
                            } else {
                                "No loco active. Spawn one below:"
                            })
                            .color(p.text_muted),
                        );
                    }

                    ui.add_space(6.0);
                    // Quick Spawn Buttons
                    ui.label(RichText::new(if is_de { "Lok / Auto platzieren" } else { "Spawn Loco / Car" }).strong().size(11.5));
                    ui.horizontal_wrapped(|ui| {
                        for (kind, lbl) in [
                            (VehicleKind::ElectricLoco, "BR 103"),
                            (VehicleKind::SteamLoco, "BR 01"),
                            (VehicleKind::DieselLoco, "BR 218"),
                            (VehicleKind::Sedan, "PKW"),
                            (VehicleKind::Truck, "LKW"),
                            (VehicleKind::Bus, "Bus"),
                        ] {
                            if ui.small_button(format!("+ {lbl}")).clicked() {
                                spawn_req = Some(kind);
                            }
                        }
                    });

                    // Live Turnout Switcher right inside Cab Window
                    let turnouts: Vec<(String, String, usize, bool)> = app
                        .layout
                        .pieces
                        .iter()
                        .filter_map(|pc| {
                            let e = crate::catalog::find_entry(&app.catalogs, &pc.catalog_id)?;
                            if crate::is_turnout_kind(&e.kind) && pc.connectors.len() >= 3 {
                                Some((
                                    pc.id.clone(),
                                    pc.catalog_id.clone(),
                                    crate::turnout_state_count(&e.kind, pc.connectors.len()),
                                    e.kind == "double-slip",
                                ))
                            } else {
                                None
                            }
                        })
                        .collect();
                    if !turnouts.is_empty() {
                        ui.add_space(6.0);
                        ui.separator();
                        ui.label(RichText::new(if is_de { "Weichen stellen" } else { "Switch Turnouts" }).strong().size(11.5));
                        ui.horizontal_wrapped(|ui| {
                            for (tid, cid, br_cnt, is_dkw) in turnouts.iter().take(16) {
                                let st = app.vehicle_sim.turnout_states.get(tid).copied().unwrap_or(0) % (*br_cnt).max(2);
                                let lbl = if *is_dkw {
                                    if st == 0 {
                                        format!("{cid} ×")
                                    } else {
                                        format!("{cid} )(")
                                    }
                                } else if st == 0 {
                                    format!("{cid} ↑")
                                } else if *br_cnt >= 3 && st == 1 {
                                    format!("{cid} ↖")
                                } else if *br_cnt >= 3 && st == 2 {
                                    format!("{cid} ↗")
                                } else {
                                    format!("{cid} ↱{st}")
                                };
                                let btn = egui::Button::new(RichText::new(lbl).size(11.0).color(Color32::WHITE))
                                    .fill(if st == 0 {
                                        Color32::from_rgb(2, 132, 199)
                                    } else {
                                        Color32::from_rgb(217, 119, 6)
                                    })
                                    .small();
                                if ui.add(btn).clicked() {
                                    toggle_turnout_req = Some((tid.clone(), *br_cnt));
                                }
                            }
                        });
                    }

                    draw_turntable_transfer_controls(ui, app, is_de);
                });
            });

        // Optional bottom Stellwerk pane when in Split mode (cab_view_layout == 2)
        if app.cab_view_layout == 2 {
            egui::TopBottomPanel::bottom("cab_split_stellwerk")
                .resizable(true)
                .default_height(300.0)
                .min_height(160.0)
                .frame(Frame::NONE)
                .show(vp_ctx, |ui| {
                    super::stellwerk_panel::draw_stellwerk_canvas(ui, app);
                });
        }

        // Main Area: 3D Cab View (modes 0 & 2) OR Full-Window Stellwerk (mode 1)
        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(vp_ctx, |ui| {
                if app.cab_view_layout == 1 {
                    super::stellwerk_panel::draw_stellwerk_canvas(ui, app);
                } else if let Some(idx) = active_idx {
                    let v = &app.vehicle_sim.vehicles[idx];
                    let sf = crate::model_tools::scale_factor_from_h0(&app.layout.scale);
                    let def = v.kind.def();
                    let pose = v.lead_pose();
                    let len = pose
                        .map(|p| p.length_mm as f32)
                        .unwrap_or_else(|| (def.length_h0_mm * sf) as f32);
                    let height = pose
                        .map(|p| p.height_mm as f32)
                        .unwrap_or_else(|| (def.height_h0_mm * sf) as f32);
                    let y_off = if v.domain == VehicleDomain::Rail {
                        crate::vehicle_sim::RAIL_TOP_Y_MM as f32
                    } else {
                        crate::vehicle_sim::ROAD_TOP_Y_MM as f32
                    };

                    let driving_back = v.velocity_mm_s < -1.0 || (v.velocity_mm_s.abs() <= 1.0 && v.reverse);
                    let fwd_sign = if driving_back { -1.0_f32 } else { 1.0_f32 };
                    let yaw_rad = (v.heading_deg as f32).to_radians();
                    let pitch_rad = v.pitch_rad as f32;
                    let cp = pitch_rad.cos();
                    let sp = pitch_rad.sin();
                    // Forward unit vector in 3D (X, Y=up, Z=2D_y)
                    let fx = yaw_rad.cos() * cp * fwd_sign;
                    let fy = sp * fwd_sign;
                    let fz = yaw_rad.sin() * cp * fwd_sign;

                    let base = [v.world_x as f32, v.world_z as f32 + y_off, v.world_y as f32];
                    let (eye, look_dist, fov_deg) = match app.cab_camera_mode {
                        CabCameraMode::Cab => (
                            [
                                base[0] + fx * (len * 0.54),
                                base[1] + height * 0.68 + fy * (len * 0.54),
                                base[2] + fz * (len * 0.54),
                            ],
                            420.0_f32,
                            60.0_f32,
                        ),
                        CabCameraMode::Roof => (
                            [
                                base[0] - fx * (len * 0.15),
                                base[1] + height * 1.38 - fy * (len * 0.15),
                                base[2] - fz * (len * 0.15),
                            ],
                            450.0_f32,
                            56.0_f32,
                        ),
                        CabCameraMode::Chase => (
                            [
                                base[0] - fx * (len * 1.85),
                                base[1] + height * 2.8 - fy * (len * 1.85),
                                base[2] - fz * (len * 1.85),
                            ],
                            380.0_f32,
                            50.0_f32,
                        ),
                    };

                    // Apply optional driver look-around yaw/pitch offset
                    let look_yaw = yaw_rad + if driving_back { std::f32::consts::PI } else { 0.0 } + app.cab_look_yaw;
                    let look_pitch = (pitch_rad * fwd_sign + app.cab_look_pitch).clamp(-1.1, 1.1);
                    let lcp = look_pitch.cos();
                    let target = [
                        eye[0] + look_yaw.cos() * lcp * look_dist,
                        eye[1] + look_pitch.sin() * look_dist,
                        eye[2] + look_yaw.sin() * lcp * look_dist,
                    ];

                    let kmh = v.scale_speed_kmh(&app.layout.scale);
                    draw_cab_viewport_3d(
                        ui,
                        &app.scene_3d_bundle,
                        vehicles_mesh.clone(),
                        eye,
                        target,
                        fov_deg,
                        app.viewer3d_env,
                        app.time_of_day,
                        app.dark_mode,
                        app.cab_camera_mode == CabCameraMode::Cab,
                        &mut app.cab_look_yaw,
                        &mut app.cab_look_pitch,
                        Some((&v.name, kmh, v.grade_pct, driving_back, v.headlights)),
                        is_de,
                    );
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(120.0);
                            ui.label(
                                RichText::new(if is_de {
                                    "🚂 Fuehrerstandsansicht — Keine Lok auf den Gleisen"
                                } else {
                                    "🚂 Cab View — No locomotive on the tracks yet"
                                })
                                .size(18.0)
                                .strong(),
                            );
                            ui.add_space(12.0);
                            if ui
                                .button(
                                    RichText::new(if is_de {
                                        "▶ E-Lok BR 103 platzieren & starten"
                                    } else {
                                        "▶ Spawn Electric Loco BR 103 & Start"
                                    })
                                    .size(14.0),
                                )
                                .clicked()
                            {
                                spawn_req = Some(VehicleKind::ElectricLoco);
                            }
                        });
                    });
                }
            });

        // Apply deferred actions
        if let Some(kind) = spawn_req {
            let pref = if app.selected.len() == 1 {
                app.selected.iter().next().cloned()
            } else {
                None
            };
            match app
                .vehicle_sim
                .spawn_vehicle(&app.layout, &app.catalogs, kind, pref.as_deref())
            {
                Ok(_) => app.vehicle_sim.running = true,
                Err(e) => app.status_message = Some(e),
            }
        }
        if let Some(rid) = remove_id {
            app.vehicle_sim.remove_vehicle(&rid);
        }
        if let Some((x, y)) = focus_pt {
            app.focus_world_point(x, y);
        }
        if let Some((vid, wk)) = add_wagon_req {
            app.vehicle_sim.add_wagon(&vid, wk, &app.layout, &app.catalogs);
        }
        if let Some(vid) = rem_wagon_id {
            app.vehicle_sim.remove_last_wagon(&vid, &app.layout, &app.catalogs);
        }
        if let Some((tid, cnt)) = toggle_turnout_req {
            app.vehicle_sim.toggle_turnout(&tid, cnt);
        }

        if (app.vehicle_sim.running && !app.vehicle_sim.vehicles.is_empty())
            || app.vehicle_sim.has_moving_bridges()
        {
            vp_ctx.request_repaint();
        }
    });
}

fn draw_turntable_transfer_controls(ui: &mut egui::Ui, app: &mut RailbenchApp, is_de: bool) {
    let specials: Vec<(String, String, String)> = app
        .layout
        .pieces
        .iter()
        .filter_map(|pc| {
            let e = crate::catalog::find_entry(&app.catalogs, &pc.catalog_id)?;
            if matches!(e.kind.as_str(), "turntable" | "transfer-table") {
                Some((pc.id.clone(), pc.catalog_id.clone(), e.kind.clone()))
            } else {
                None
            }
        })
        .collect();
    if specials.is_empty() {
        return;
    }

    ui.add_space(6.0);
    ui.separator();
    ui.label(
        RichText::new(if is_de {
            "Drehscheibe & Schiebebuehne"
        } else {
            "Turntable & Transfer Table"
        })
        .strong()
        .size(12.0),
    );

    let mut tt_step: Option<(String, String, isize)> = None;
    let mut tt_set: Option<(String, f64)> = None;
    let mut tr_step: Option<(String, String, isize)> = None;

    for (pid, cid, kind) in &specials {
        if kind == "turntable" {
            let cur_deg = app.vehicle_sim.turntable_angles_deg.get(pid).copied().unwrap_or(0.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("⟳ {cid} ({cur_deg:.0}°)")).small().strong());
                if ui
                    .small_button("↶")
                    .on_hover_text(if is_de { "Gegen Uhrzeigersinn" } else { "Rotate CCW" })
                    .clicked()
                {
                    tt_step = Some((pid.clone(), cid.clone(), -1));
                }
                if ui
                    .small_button("↷")
                    .on_hover_text(if is_de { "Im Uhrzeigersinn" } else { "Rotate CW" })
                    .clicked()
                {
                    tt_step = Some((pid.clone(), cid.clone(), 1));
                }
                if ui
                    .small_button("180°")
                    .on_hover_text(if is_de { "Lok wenden (180°)" } else { "Turn loco 180°" })
                    .clicked()
                {
                    tt_set = Some((pid.clone(), cur_deg + 180.0));
                }
            });
            if let Some(piece) = app.layout.pieces.iter().find(|p| p.id == *pid) {
                let conn_slots: Vec<(usize, f64)> = piece
                    .connectors
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.connected_to.is_some())
                    .map(|(ci, c)| {
                        let local = railbench_core::geometry::rotate(
                            railbench_core::types::Vec2 {
                                x: c.x - piece.transform.x,
                                y: c.y - piece.transform.y,
                            },
                            -piece.transform.rotation,
                        );
                        let ang = ((local.y.atan2(local.x).to_degrees() % 360.0) + 360.0) % 360.0;
                        (ci + 1, ang)
                    })
                    .collect();
                if !conn_slots.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for (slot_no, ang) in conn_slots.iter().take(10) {
                            if ui.small_button(format!("S{slot_no}")).clicked() {
                                tt_set = Some((pid.clone(), *ang));
                            }
                        }
                    });
                }
            }
        } else {
            let entry = crate::catalog::find_entry(&app.catalogs, cid);
            let zs = crate::turntables_3d::extract_transfer_track_zs(entry);
            let def_z = zs[zs.len() / 2] as f64;
            let cur_z = app.vehicle_sim.transfer_offsets_mm.get(pid).copied().unwrap_or(def_z);
            let bay = zs
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    ((**a as f64) - cur_z)
                        .abs()
                        .partial_cmp(&((**b as f64) - cur_z).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i + 1)
                .unwrap_or(1);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("⇄ {cid} ({}/{})", bay, zs.len()))
                        .small()
                        .strong(),
                );
                if ui
                    .small_button("◀")
                    .on_hover_text(if is_de { "Vorheriges Gleis" } else { "Previous bay" })
                    .clicked()
                {
                    tr_step = Some((pid.clone(), cid.clone(), -1));
                }
                if ui
                    .small_button("▶")
                    .on_hover_text(if is_de { "Naechstes Gleis" } else { "Next bay" })
                    .clicked()
                {
                    tr_step = Some((pid.clone(), cid.clone(), 1));
                }
            });
        }
    }

    if let Some((pid, cid, delta)) = tt_step {
        let entry = crate::catalog::find_entry(&app.catalogs, &cid).cloned();
        app.vehicle_sim.step_turntable(&pid, entry.as_ref(), delta);
    }
    if let Some((pid, deg)) = tt_set {
        app.vehicle_sim.set_turntable_target(&pid, deg);
    }
    if let Some((pid, cid, delta)) = tr_step {
        let entry = crate::catalog::find_entry(&app.catalogs, &cid).cloned();
        app.vehicle_sim.step_transfer_table(&pid, entry.as_ref(), delta);
    }
}
