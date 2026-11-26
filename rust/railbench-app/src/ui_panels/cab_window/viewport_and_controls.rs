use super::*;

/// Returns `(fwd_frac, height_frac, side_frac, default_pitch_rad)` for `CabCameraMode::Cab`
/// so each locomotive looks over its authentic hood/boiler without clipping into cab walls.
pub(crate) fn cab_camera_offsets(kind: VehicleKind, driving_back: bool) -> (f32, f32, f32, f32) {
    match kind {
        VehicleKind::Br243 => (0.505, 0.78, 0.0, -0.05),
        VehicleKind::Br96Mallet => {
            if driving_back {
                (0.395, 0.88, 0.0, -0.05)
            } else {
                (-0.214, 0.90, 0.34, -0.04)
            }
        }
        VehicleKind::V60Diesel => {
            if driving_back {
                (0.190, 0.89, 0.14, -0.05)
            } else {
                (0.068, 0.89, 0.14, -0.05)
            }
        }
        _ => (0.495, 0.80, 0.0, -0.04),
    }
}

pub(super) fn draw_active_cab_viewport(
    ui: &mut egui::Ui,
    app: &mut RailbenchApp,
    vehicles_mesh: Option<std::sync::Arc<SceneMesh3D>>,
    idx: usize,
    is_de: bool,
) {
    let Some(v) = app.vehicle_sim.vehicles.get(idx) else {
        return;
    };
    let sf = crate::model_tools::scale_factor_from_h0(&app.layout.scale);
    let def = v.kind.def();
    let pose = v.lead_pose();
    let len = pose
        .map(|p| p.length_mm as f32)
        .unwrap_or_else(|| (def.length_h0_mm * sf) as f32);
    let width = pose
        .map(|p| p.width_mm as f32)
        .unwrap_or_else(|| (def.width_h0_mm * sf) as f32);
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
    let fx = yaw_rad.cos() * cp * fwd_sign;
    let fy = sp * fwd_sign;
    let fz = yaw_rad.sin() * cp * fwd_sign;
    // Lateral right vector in local travel direction (+Z in loco coords when moving forward)
    let rx = -yaw_rad.sin() * fwd_sign;
    let rz = yaw_rad.cos() * fwd_sign;

    let base = [v.world_x as f32, v.world_z as f32 + y_off, v.world_y as f32];
    let (eye, look_dist, fov_deg, default_pitch) = match app.cab_camera_mode {
        CabCameraMode::Cab => {
            let (fwd_frac, height_frac, side_frac, cab_pitch) =
                cab_camera_offsets(v.kind, driving_back);
            (
                [
                    base[0] + fx * (len * fwd_frac) + rx * (width * side_frac),
                    base[1] + height * height_frac + fy * (len * fwd_frac),
                    base[2] + fz * (len * fwd_frac) + rz * (width * side_frac),
                ],
                420.0_f32,
                64.0_f32,
                cab_pitch,
            )
        }
        CabCameraMode::Roof => (
            [
                base[0] - fx * (len * 0.15),
                base[1] + height * 1.38 - fy * (len * 0.15),
                base[2] - fz * (len * 0.15),
            ],
            450.0_f32,
            56.0_f32,
            -0.06_f32,
        ),
        CabCameraMode::Chase => (
            [
                base[0] - fx * (len * 1.85),
                base[1] + height * 2.8 - fy * (len * 1.85),
                base[2] - fz * (len * 1.85),
            ],
            380.0_f32,
            50.0_f32,
            -0.16_f32,
        ),
    };

    let look_yaw = yaw_rad + if driving_back { std::f32::consts::PI } else { 0.0 } + app.cab_look_yaw;
    let look_pitch = (pitch_rad * fwd_sign + default_pitch + app.cab_look_pitch).clamp(-1.1, 1.1);
    let lcp = look_pitch.cos();
    let target = [
        eye[0] + look_yaw.cos() * lcp * look_dist,
        eye[1] + look_pitch.sin() * look_dist,
        eye[2] + look_yaw.sin() * lcp * look_dist,
    ];

    let kmh = v.scale_speed_kmh(&app.layout.scale);
    let max_kmh = (def.max_speed_mm_s * 0.087 * 3.6).max(40.0);
    let hud = CabHudInfo {
        name: &v.name,
        kind: v.kind,
        kmh,
        max_kmh,
        throttle: v.throttle,
        grade_pct: v.grade_pct,
        reverse: v.reverse,
        headlights: v.headlights,
        pantograph_up: v.pantograph_up,
        steam_on: v.steam_on,
        blocked_ahead: v.blocked_ahead,
        sim_running: app.vehicle_sim.running,
        coupled_count: v.coupled_cars.len(),
    };

    let interaction = draw_cab_viewport_3d(
        ui,
        &app.scene_3d_bundle,
        vehicles_mesh,
        eye,
        target,
        fov_deg,
        app.viewer3d_env,
        app.time_of_day,
        app.dark_mode,
        app.low_gpu_mode,
        app.cab_camera_mode == CabCameraMode::Cab,
        &mut app.cab_look_yaw,
        &mut app.cab_look_pitch,
        Some(hud),
        is_de,
    );

    if let Some(new_thr) = interaction.set_throttle {
        if let Some(veh) = app.vehicle_sim.vehicles.get_mut(idx) {
            veh.throttle = new_thr.clamp(0.0, 1.0);
            if veh.throttle > 0.0 {
                app.vehicle_sim.running = true;
            }
        }
    }
    if interaction.toggle_reverse {
        if let Some(veh) = app.vehicle_sim.vehicles.get_mut(idx) {
            veh.reverse = !veh.reverse;
        }
    }
    if interaction.toggle_headlights {
        if let Some(veh) = app.vehicle_sim.vehicles.get_mut(idx) {
            veh.headlights = !veh.headlights;
        }
    }
    if interaction.emergency_stop {
        app.vehicle_sim.emergency_stop_all();
    }
    if interaction.uncouple_last {
        if let Some(vid) = app.vehicle_sim.vehicles.get(idx).map(|v| v.id.clone()) {
            app.vehicle_sim.uncouple_last_wagon(&vid, &app.layout, &app.catalogs);
        }
    }
}

pub(super) fn draw_empty_cab_state(
    ui: &mut egui::Ui,
    p: crate::theme::RailbenchPalette,
    dark_mode: bool,
    is_de: bool,
    spawn_req: &mut Option<VehicleKind>,
) {
    let avail = ui.available_size().max(egui::vec2(120.0, 120.0));
    let (rect, _) = ui.allocate_exact_size(avail, Sense::hover());
    let bg_fill = if dark_mode {
        Color32::from_rgb(13, 19, 34)
    } else {
        p.canvas_bg
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, bg_fill);

    // Subtle technical backdrop grid across the empty viewport
    let grid_col = if dark_mode {
        Color32::from_rgba_unmultiplied(51, 65, 85, 36)
    } else {
        Color32::from_rgba_unmultiplied(138, 126, 108, 34)
    };
    let step = 36.0_f32;
    let mut gx = rect.min.x;
    while gx <= rect.max.x {
        painter.line_segment(
            [egui::Pos2::new(gx, rect.min.y), egui::Pos2::new(gx, rect.max.y)],
            Stroke::new(1.0, grid_col),
        );
        gx += step;
    }
    let mut gy = rect.min.y;
    while gy <= rect.max.y {
        painter.line_segment(
            [egui::Pos2::new(rect.min.x, gy), egui::Pos2::new(rect.max.x, gy)],
            Stroke::new(1.0, grid_col),
        );
        gy += step;
    }

    let card_w = 420.0_f32.min((rect.width() - 28.0).max(240.0));
    let card_h = 228.0_f32.min((rect.height() - 20.0).max(180.0));
    let card_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(card_w, card_h));

    ui.scope_builder(egui::UiBuilder::new().max_rect(card_rect), |ui| {
        Frame::new()
            .fill(p.bg_card)
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::symmetric(24, 18))
            .stroke(Stroke::new(1.0_f32, p.border_main))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    let (icon_r, _) = ui.allocate_exact_size(egui::vec2(42.0, 42.0), Sense::hover());
                    ui.painter().rect(
                        icon_r,
                        CornerRadius::same(10),
                        p.bg_sidebar,
                        Stroke::new(1.0_f32, p.border_main),
                        egui::StrokeKind::Inside,
                    );
                    paint_icon(ui.painter(), icon_r.shrink(10.0), Icon::Train, p.primary);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(if is_de {
                            "Fuehrerstandsansicht bereit"
                        } else {
                            "3D Cab View Ready"
                        })
                        .strong()
                        .size(16.0)
                        .color(p.text_main),
                    );
                    ui.add_space(3.0);
                    ui.label(
                        RichText::new(if is_de {
                            "Platziere eine Lokomotive auf den Gleisen, um die Mitfahrt zu starten."
                        } else {
                            "Spawn a locomotive onto your tracks to start driving in 3D."
                        })
                        .size(12.0)
                        .color(p.text_muted),
                    );
                    ui.add_space(12.0);
                    if ui
                        .add_sized(
                            [(card_w - 48.0).min(290.0), 34.0],
                            egui::Button::new(
                                RichText::new(if is_de {
                                    "E-Lok BR 243 platzieren & starten"
                                } else {
                                    "Spawn BR 243 Electric & Start"
                                })
                                .strong()
                                .size(12.5)
                                .color(Color32::WHITE),
                            )
                            .fill(p.primary_hover)
                            .stroke(Stroke::new(1.0_f32, p.primary))
                            .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        *spawn_req = Some(VehicleKind::Br243);
                    }
                    ui.add_space(8.0);
                    ui.columns(2, |cols| {
                        if cols[0]
                            .add_sized(
                                [cols[0].available_width(), 26.0],
                                egui::Button::new(RichText::new("+ BR 96 Mallet").size(11.5).color(p.text_main))
                                    .fill(p.bg_sidebar)
                                    .stroke(Stroke::new(1.0_f32, p.border_main))
                                    .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            *spawn_req = Some(VehicleKind::Br96Mallet);
                        }
                        if cols[1]
                            .add_sized(
                                [cols[1].available_width(), 26.0],
                                egui::Button::new(RichText::new("+ V 60 Diesel").size(11.5).color(p.text_main))
                                    .fill(p.bg_sidebar)
                                    .stroke(Stroke::new(1.0_f32, p.border_main))
                                    .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            *spawn_req = Some(VehicleKind::V60Diesel);
                        }
                    });
                });
            });
    });
}


pub(super) fn draw_uncoupler_controls(
    ui: &mut egui::Ui,
    app: &RailbenchApp,
    p: crate::theme::RailbenchPalette,
    is_de: bool,
    actions: &mut CabActions,
) {
    let uncouplers: Vec<(String, String)> = app
        .layout
        .pieces
        .iter()
        .filter_map(|pc| {
            let e = crate::catalog::find_entry(&app.catalogs, &pc.catalog_id)?;
            if crate::catalog::is_uncoupler_entry(e) {
                Some((pc.id.clone(), pc.catalog_id.clone()))
            } else {
                None
            }
        })
        .collect();
    if uncouplers.is_empty() {
        return;
    }

    draw_desk_card(ui, p, |ui| {
        ui.label(
            RichText::new(if is_de {
                format!("ENTKUPPLUNGSGLEISE ({})", uncouplers.len())
            } else {
                format!("UNCOUPLER TRACKS ({})", uncouplers.len())
            })
            .strong()
            .size(10.5)
            .color(p.text_muted),
        );

        for (uid, cid) in &uncouplers {
            let raised = app.vehicle_sim.is_uncoupler_raised(uid);
            let armed = app.vehicle_sim.armed_uncouplers.contains(uid);
            let ready_coupler = app
                .vehicle_sim
                .find_coupler_over_piece(uid, &app.layout, &app.catalogs)
                .is_some();

            ui.horizontal(|ui| {
                ui.label(RichText::new(cid).strong().size(11.5).color(p.text_main));
                if ready_coupler {
                    Frame::new()
                        .fill(Color32::from_rgb(22, 163, 74))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(5, 1))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(if is_de { "KUPPLUNG" } else { "READY" })
                                    .strong()
                                    .size(9.0)
                                    .color(Color32::WHITE),
                            );
                        });
                } else if raised {
                    Frame::new()
                        .fill(Color32::from_rgb(217, 119, 6))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(5, 1))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(if is_de { "AKTIV" } else { "ACTIVE" })
                                    .strong()
                                    .size(9.0)
                                    .color(Color32::WHITE),
                            );
                        });
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Auto")
                                    .strong()
                                    .size(10.0)
                                    .color(if armed { Color32::WHITE } else { p.text_main }),
                            )
                            .fill(if armed {
                                Color32::from_rgb(124, 58, 237)
                            } else {
                                p.bg_sidebar
                            })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if armed {
                                    Color32::from_rgb(167, 139, 250)
                                } else {
                                    p.border_main
                                },
                            ))
                            .min_size(egui::vec2(42.0, 23.0))
                            .corner_radius(CornerRadius::same(5)),
                        )
                        .on_hover_text(if is_de {
                            "Dauerbetrieb: Jede ueberfahrende Kupplung automatisch loesen"
                        } else {
                            "Auto mode: automatically uncouple any train passing over this track"
                        })
                        .clicked()
                    {
                        actions.toggle_uncoupler_armed = Some(uid.clone());
                    }

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(if is_de { "Entkuppeln" } else { "Uncouple" })
                                    .strong()
                                    .size(10.5)
                                    .color(Color32::WHITE),
                            )
                            .fill(if raised {
                                Color32::from_rgb(217, 119, 6)
                            } else {
                                p.primary_hover
                            })
                            .stroke(Stroke::new(1.0_f32, p.primary))
                            .min_size(egui::vec2(76.0, 23.0))
                            .corner_radius(CornerRadius::same(5)),
                        )
                        .on_hover_text(if is_de {
                            "Entkupplerbohle anheben & Kupplung loesen"
                        } else {
                            "Raise uncoupler ramp & disconnect coupler"
                        })
                        .clicked()
                    {
                        actions.trigger_uncoupler = Some(uid.clone());
                    }
                });
            });
        }
    });
}

pub(super) fn draw_turntable_transfer_controls(
    ui: &mut egui::Ui,
    app: &mut RailbenchApp,
    p: crate::theme::RailbenchPalette,
    is_de: bool,
) {
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

    let mut tt_step: Option<(String, String, isize)> = None;
    let mut tt_set: Option<(String, f64)> = None;
    let mut tr_step: Option<(String, String, isize)> = None;

    draw_desk_card(ui, p, |ui| {
        ui.label(
            RichText::new(if is_de {
                "DREHSCHEIBE & SCHIEBEBUEHNE"
            } else {
                "TURNTABLE & TRANSFER TABLE"
            })
            .strong()
            .size(10.5)
            .color(p.text_muted),
        );

        for (pid, cid, kind) in &specials {
            if kind == "turntable" {
                let cur_deg = app.vehicle_sim.turntable_angles_deg.get(pid).copied().unwrap_or(0.0);
                let norm_cur = ((cur_deg % 360.0) + 360.0) % 360.0;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{cid} ({cur_deg:.0}°)")).small().strong().color(p.text_main));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for (lbl, tip_de, tip_en, action_kind) in [
                            ("180°", "Lok wenden (180°)", "Turn loco 180°", 0u8),
                            ("CW", "Im Uhrzeigersinn", "Rotate CW", 1u8),
                            ("CCW", "Gegen Uhrzeigersinn", "Rotate CCW", 2u8),
                        ] {
                            let min_w = if lbl == "180°" { 38.0 } else { 34.0 };
                            if ui
                                .add(
                                    egui::Button::new(RichText::new(lbl).strong().size(10.0).color(p.text_main))
                                        .fill(p.bg_sidebar)
                                        .stroke(Stroke::new(1.0_f32, p.border_main))
                                        .min_size(egui::vec2(min_w, 23.0))
                                        .corner_radius(CornerRadius::same(5)),
                                )
                                .on_hover_text(if is_de { tip_de } else { tip_en })
                                .clicked()
                            {
                                match action_kind {
                                    0 => tt_set = Some((pid.clone(), cur_deg + 180.0)),
                                    1 => tt_step = Some((pid.clone(), cid.clone(), 1)),
                                    _ => tt_step = Some((pid.clone(), cid.clone(), -1)),
                                }
                            }
                        }
                    });
                });
                if let Some(piece) = app.layout.pieces.iter().find(|pc| pc.id == *pid) {
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
                            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                            for (slot_no, ang) in conn_slots.iter().take(12) {
                                let diff = (((norm_cur - ang + 540.0) % 360.0) - 180.0).abs();
                                let aligned = diff <= 4.0 || (diff - 180.0).abs() <= 4.0;
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(format!("S{slot_no}"))
                                                .strong()
                                                .size(10.0)
                                                .color(if aligned { Color32::WHITE } else { p.text_main }),
                                        )
                                        .fill(if aligned { p.primary_hover } else { p.bg_sidebar })
                                        .stroke(Stroke::new(1.0_f32, if aligned { p.primary } else { p.border_main }))
                                        .min_size(egui::vec2(34.0, 23.0))
                                        .corner_radius(CornerRadius::same(5)),
                                    )
                                    .clicked()
                                {
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
                        RichText::new(format!("{cid} ({}/{})", bay, zs.len()))
                            .small()
                            .strong()
                            .color(p.text_main),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new(">").strong().size(11.0).color(p.text_main))
                                    .fill(p.bg_sidebar)
                                    .stroke(Stroke::new(1.0_f32, p.border_main))
                                    .min_size(egui::vec2(28.0, 23.0))
                                    .corner_radius(CornerRadius::same(5)),
                            )
                            .on_hover_text(if is_de { "Naechstes Gleis" } else { "Next bay" })
                            .clicked()
                        {
                            tr_step = Some((pid.clone(), cid.clone(), 1));
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new("<").strong().size(11.0).color(p.text_main))
                                    .fill(p.bg_sidebar)
                                    .stroke(Stroke::new(1.0_f32, p.border_main))
                                    .min_size(egui::vec2(28.0, 23.0))
                                    .corner_radius(CornerRadius::same(5)),
                            )
                            .on_hover_text(if is_de { "Vorheriges Gleis" } else { "Previous bay" })
                            .clicked()
                        {
                            tr_step = Some((pid.clone(), cid.clone(), -1));
                        }
                    });
                });
            }
        }
    });

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

