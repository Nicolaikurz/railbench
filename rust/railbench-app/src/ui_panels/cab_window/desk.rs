use super::*;

pub(super) fn draw_cab_control_desk(
    vp_ctx: &egui::Context,
    app: &mut RailbenchApp,
    p: crate::theme::RailbenchPalette,
    is_de: bool,
    active_idx: Option<usize>,
    actions: &mut CabActions,
) {
    egui::SidePanel::right("cab_control_desk")
        .default_width(308.0)
        .width_range(260.0..=440.0)
        .resizable(!app.bars_locked)
        .frame(
            Frame::new()
                .fill(p.bg_sidebar)
                .inner_margin(Margin::same(10))
                .stroke(Stroke::new(1.0_f32, p.border_main)),
        )
        .show(vp_ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                // 1. Fleet Selector Card
                if !app.vehicle_sim.vehicles.is_empty() {
                    draw_desk_card(ui, p, |ui| {
                        let scale_str = &app.layout.scale;
                        ui.label(
                            RichText::new(if is_de {
                                format!("FUHRPARK ({})", app.vehicle_sim.vehicles.len())
                            } else {
                                format!("FLEET ({})", app.vehicle_sim.vehicles.len())
                            })
                            .strong()
                            .size(10.5)
                            .color(p.text_muted),
                        );
                        ui.horizontal_wrapped(|ui| {
                            for (i, v) in app.vehicle_sim.vehicles.iter().enumerate() {
                                let is_cur = active_idx == Some(i);
                                let kmh = v.scale_speed_kmh(scale_str);
                                let lbl = if !v.kind.is_locomotive() && v.domain == VehicleDomain::Rail {
                                    format!("{} · {}", v.name, if is_de { "ABGEST." } else { "PARKED" })
                                } else if v.blocked_ahead {
                                    format!("{} · {}", v.name, if is_de { "HALT" } else { "BLOCKED" })
                                } else {
                                    format!("{} · {:.0} km/h", v.name, kmh)
                                };
                                let fill = if is_cur {
                                    p.primary_hover
                                } else if v.blocked_ahead {
                                    Color32::from_rgba_unmultiplied(245, 158, 11, 36)
                                } else {
                                    p.bg_sidebar
                                };
                                let txt_col = if is_cur { Color32::WHITE } else { p.text_main };
                                let border = if is_cur {
                                    p.primary
                                } else if v.blocked_ahead {
                                    Color32::from_rgb(245, 158, 11)
                                } else {
                                    p.border_main
                                };
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new(lbl).strong().size(11.0).color(txt_col))
                                            .fill(fill)
                                            .stroke(Stroke::new(1.0_f32, border))
                                            .corner_radius(CornerRadius::same(7)),
                                    )
                                    .clicked()
                                {
                                    app.vehicle_sim.selected_vehicle_id = Some(v.id.clone());
                                    app.cab_look_yaw = 0.0;
                                    app.cab_look_pitch = 0.0;
                                }
                            }
                        });
                    });
                }

                // 2. Active Vehicle Driver's Desk Card
                if let Some(idx) = active_idx.filter(|&i| i < app.vehicle_sim.vehicles.len()) {
                    let scale_str = app.layout.scale.clone();
                    let sim_running = app.vehicle_sim.running;
                    let has_nearby_couple = app
                        .vehicle_sim
                        .vehicles
                        .get(idx)
                        .filter(|v| v.kind.is_locomotive() && v.coupled_cars.len() < 8)
                        .and_then(|v| app.vehicle_sim.find_nearby_coupleable_cut(&v.id, &app.layout))
                        .is_some();
                    let mut start_sim = false;
                    let v = &mut app.vehicle_sim.vehicles[idx];
                    let kmh = v.scale_speed_kmh(&scale_str);
                    let tons = v.total_mass_tons();

                    draw_desk_card(ui, p, |ui| {
                        // Header row: Name + status badge + Focus + Delete
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&v.name).strong().size(14.0).color(p.text_main));
                            let (st_lbl, st_col) = if !v.kind.is_locomotive() && v.domain == VehicleDomain::Rail {
                                (if is_de { "ABGESTELLT" } else { "PARKED" }, Color32::from_rgb(71, 85, 105))
                            } else if v.blocked_ahead {
                                (if is_de { "HALT" } else { "BLOCKED" }, Color32::from_rgb(217, 119, 6))
                            } else if sim_running && kmh > 0.5 {
                                (if is_de { "FAHRT" } else { "MOVING" }, Color32::from_rgb(22, 163, 74))
                            } else {
                                (if is_de { "STAND" } else { "STOPPED" }, Color32::from_rgb(100, 116, 139))
                            };
                            Frame::new()
                                .fill(st_col)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(5, 1))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(st_lbl).strong().size(9.5).color(Color32::WHITE));
                                });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if icon_button_sized(
                                    ui,
                                    Icon::Trash,
                                    egui::vec2(22.0, 22.0),
                                    false,
                                    p.bg_sidebar,
                                    Color32::from_rgb(220, 38, 38),
                                    Color32::from_rgb(220, 38, 38),
                                    p.text_sub,
                                    if is_de { "Fahrzeug entfernen" } else { "Remove vehicle" },
                                )
                                .clicked()
                                {
                                    actions.remove_id = Some(v.id.clone());
                                }
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new("2D").strong().size(10.5).color(p.text_main))
                                            .fill(p.bg_sidebar)
                                            .stroke(Stroke::new(1.0_f32, p.border_main))
                                            .min_size(egui::vec2(26.0, 22.0))
                                            .corner_radius(CornerRadius::same(5)),
                                    )
                                    .on_hover_text(if is_de { "In 2D-Ansicht fokussieren" } else { "Focus in 2D view" })
                                    .clicked()
                                {
                                    actions.focus_pt = Some((v.world_x, v.world_y));
                                }
                            });
                        });

                        // 3-Tile Telemetry Strip (Speed / Grade / Mass)
                        ui.columns(3, |cols| {
                            let grade_col = if v.grade_pct.abs() > 3.5 {
                                p.text_danger
                            } else if v.grade_pct.abs() > 1.5 {
                                p.text_warning
                            } else {
                                p.text_main
                            };
                            draw_telemetry_tile(&mut cols[0], p, if is_de { "TEMPO" } else { "SPEED" }, format!("{kmh:.0} km/h"), p.primary);
                            draw_telemetry_tile(&mut cols[1], p, if is_de { "STEIG." } else { "GRADE" }, format!("{:+.1}%", v.grade_pct), grade_col);
                            draw_telemetry_tile(&mut cols[2], p, if is_de { "MASSE" } else { "MASS" }, format!("{tons:.0} t"), p.text_main);
                        });

                        if !v.kind.is_locomotive() && v.domain == VehicleDomain::Rail {
                            ui.label(
                                RichText::new(if is_de {
                                    format!("Abgestellte Wagengruppe ({} Wagen). Mit einer Lok heranfahren und 'Ankuppeln' (C) klicken.", 1 + v.coupled_cars.len())
                                } else {
                                    format!("Parked wagon cut ({} car(s)). Back a locomotive up to couple (C).", 1 + v.coupled_cars.len())
                                })
                                .size(11.0)
                                .color(p.text_muted),
                            );
                        } else {
                        // Throttle Bar + Quick Step/Stop + Slider + 5 Notch Buttons
                        let mut stop_now = false;
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(if is_de { "FAHRREGLER" } else { "THROTTLE" })
                                    .strong()
                                    .size(10.5)
                                    .color(p.text_muted),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Stop")
                                                .strong()
                                                .size(10.0)
                                                .color(Color32::WHITE),
                                        )
                                        .fill(Color32::from_rgb(185, 28, 28))
                                        .stroke(Stroke::NONE)
                                        .min_size(egui::vec2(38.0, 21.0))
                                        .corner_radius(CornerRadius::same(5)),
                                    )
                                    .on_hover_text(if is_de { "Fahrzeug sofort anhalten (Leertaste)" } else { "Stop vehicle (Space)" })
                                    .clicked()
                                {
                                    stop_now = true;
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("+10%")
                                                .strong()
                                                .size(10.0)
                                                .color(p.text_main),
                                        )
                                        .fill(p.bg_sidebar)
                                        .stroke(Stroke::new(1.0_f32, p.border_main))
                                        .min_size(egui::vec2(38.0, 21.0))
                                        .corner_radius(CornerRadius::same(5)),
                                    )
                                    .clicked()
                                {
                                    v.throttle = (v.throttle + 0.10).clamp(0.0, 1.0);
                                    if v.throttle > 0.0 {
                                        start_sim = true;
                                    }
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("-10%")
                                                .strong()
                                                .size(10.0)
                                                .color(p.text_main),
                                        )
                                        .fill(p.bg_sidebar)
                                        .stroke(Stroke::new(1.0_f32, p.border_main))
                                        .min_size(egui::vec2(38.0, 21.0))
                                        .corner_radius(CornerRadius::same(5)),
                                    )
                                    .clicked()
                                {
                                    v.throttle = (v.throttle - 0.10).clamp(0.0, 1.0);
                                }
                                ui.label(
                                    RichText::new(format!("{:.0}%", v.throttle * 100.0))
                                        .strong()
                                        .monospace()
                                        .size(11.5)
                                        .color(p.primary),
                                );
                            });
                        });
                        if stop_now {
                            v.throttle = 0.0;
                            v.velocity_mm_s = 0.0;
                            v.current_speed_mm_s = 0.0;
                        }

                        let (bar_r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 7.0), Sense::hover());
                        ui.painter().rect_filled(bar_r, 3.5, p.bg_sidebar);
                        ui.painter().rect_stroke(bar_r, 3.5, Stroke::new(1.0_f32, p.border_main), egui::StrokeKind::Inside);
                        let fill_w = (bar_r.width() * v.throttle.clamp(0.0, 1.0)).max(if v.throttle > 0.01 { 6.0 } else { 0.0 });
                        if fill_w > 0.0 {
                            let fill_r = egui::Rect::from_min_size(bar_r.min, egui::vec2(fill_w, bar_r.height()));
                            ui.painter().rect_filled(fill_r, 3.5, p.primary);
                        }

                        if ui
                            .add(
                                egui::Slider::new(&mut v.throttle, 0.0..=1.0)
                                    .show_value(false),
                            )
                            .changed()
                            && v.throttle > 0.0
                        {
                            start_sim = true;
                        }

                        ui.columns(5, |cols| {
                            for (ci, &(pct, val)) in [("0%", 0.0_f32), ("25%", 0.25), ("50%", 0.5), ("75%", 0.75), ("100%", 1.0)]
                                .iter()
                                .enumerate()
                            {
                                let active_notch = (v.throttle - val).abs() < 0.06;
                                let btn = egui::Button::new(
                                    RichText::new(pct)
                                        .strong()
                                        .size(10.5)
                                        .color(if active_notch { Color32::WHITE } else { p.text_main }),
                                )
                                .fill(if active_notch { p.primary_hover } else { p.bg_sidebar })
                                .stroke(Stroke::new(1.0_f32, if active_notch { p.primary } else { p.border_main }))
                                .corner_radius(CornerRadius::same(5));
                                if cols[ci].add_sized([cols[ci].available_width(), 24.0], btn).clicked() {
                                    v.throttle = val;
                                    if val > 0.0 {
                                        start_sim = true;
                                    }
                                }
                            }
                        });

                        // Segmented Reverser (< REV | FWD >)
                        ui.columns(2, |cols| {
                            let rev_active = v.reverse;
                            let fwd_active = !v.reverse;
                            let rev_lbl = if is_de { "< Rueckwaerts" } else { "< Reverse" };
                            let fwd_lbl = if is_de { "Vorwaerts >" } else { "Forward >" };
                            if cols[0]
                                .add_sized(
                                    [cols[0].available_width(), 26.0],
                                    egui::Button::new(
                                        RichText::new(rev_lbl)
                                            .strong()
                                            .size(11.5)
                                            .color(if rev_active { Color32::WHITE } else { p.text_main }),
                                    )
                                    .fill(if rev_active { p.primary_hover } else { p.bg_sidebar })
                                    .stroke(Stroke::new(1.0_f32, if rev_active { p.primary } else { p.border_main }))
                                    .corner_radius(CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                v.reverse = true;
                            }
                            if cols[1]
                                .add_sized(
                                    [cols[1].available_width(), 26.0],
                                    egui::Button::new(
                                        RichText::new(fwd_lbl)
                                            .strong()
                                            .size(11.5)
                                            .color(if fwd_active { Color32::WHITE } else { p.text_main }),
                                    )
                                    .fill(if fwd_active { p.primary_hover } else { p.bg_sidebar })
                                    .stroke(Stroke::new(1.0_f32, if fwd_active { p.primary } else { p.border_main }))
                                    .corner_radius(CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                v.reverse = false;
                            }
                        });

                        // Feature Toggle Pills (Headlights, Pantograph/Steam)
                        let has_aux = matches!(v.kind, VehicleKind::Br243 | VehicleKind::Br96Mallet);
                        ui.columns(if has_aux { 2 } else { 1 }, |cols| {
                            let light_lbl = if v.headlights {
                                if is_de { "Spitzenlicht: AN" } else { "Headlights: ON" }
                            } else if is_de {
                                "Spitzenlicht: AUS"
                            } else {
                                "Headlights: OFF"
                            };
                            if cols[0]
                                .add_sized(
                                    [cols[0].available_width(), 24.0],
                                    egui::Button::new(
                                        RichText::new(light_lbl)
                                            .strong()
                                            .size(11.0)
                                            .color(if v.headlights { Color32::WHITE } else { p.text_main }),
                                    )
                                    .fill(if v.headlights { Color32::from_rgb(202, 138, 4) } else { p.bg_sidebar })
                                    .stroke(Stroke::new(
                                        1.0_f32,
                                        if v.headlights { Color32::from_rgb(234, 179, 8) } else { p.border_main },
                                    ))
                                    .corner_radius(CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                v.headlights = !v.headlights;
                            }

                            if has_aux {
                                match v.kind {
                                    VehicleKind::Br243 => {
                                        let p_lbl = if is_de { "Stromabnehmer" } else { "Pantograph" };
                                        if cols[1]
                                            .add_sized(
                                                [cols[1].available_width(), 24.0],
                                                egui::Button::new(
                                                    RichText::new(p_lbl)
                                                        .strong()
                                                        .size(11.0)
                                                        .color(if v.pantograph_up { Color32::WHITE } else { p.text_main }),
                                                )
                                                .fill(if v.pantograph_up { p.primary_hover } else { p.bg_sidebar })
                                                .stroke(Stroke::new(1.0_f32, if v.pantograph_up { p.primary } else { p.border_main }))
                                                .corner_radius(CornerRadius::same(6)),
                                            )
                                            .clicked()
                                        {
                                            v.pantograph_up = !v.pantograph_up;
                                        }
                                    }
                                    VehicleKind::Br96Mallet => {
                                        let s_lbl = if is_de { "Dampf" } else { "Steam" };
                                        if cols[1]
                                            .add_sized(
                                                [cols[1].available_width(), 24.0],
                                                egui::Button::new(
                                                    RichText::new(s_lbl)
                                                        .strong()
                                                        .size(11.0)
                                                        .color(if v.steam_on { Color32::WHITE } else { p.text_main }),
                                                )
                                                .fill(if v.steam_on { p.primary_hover } else { p.bg_sidebar })
                                                .stroke(Stroke::new(1.0_f32, if v.steam_on { p.primary } else { p.border_main }))
                                                .corner_radius(CornerRadius::same(6)),
                                            )
                                            .clicked()
                                        {
                                            v.steam_on = !v.steam_on;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        });

                        // Consist Wagons (Add, Disconnect at any coupler, Re-couple nearby)
                        if v.kind.is_locomotive() {
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{}: {}",
                                        if is_de { "WAGEN" } else { "WAGONS" },
                                        v.coupled_cars.len()
                                    ))
                                    .strong()
                                    .size(10.5)
                                    .color(p.text_muted),
                                );
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(RichText::new(if is_de { "+ Gueter" } else { "+ Freight" }).size(10.5).color(p.text_main))
                                                .fill(p.bg_sidebar)
                                                .stroke(Stroke::new(1.0_f32, p.border_main))
                                                .min_size(egui::vec2(62.0, 23.0))
                                                .corner_radius(CornerRadius::same(5)),
                                        )
                                        .clicked()
                                    {
                                        actions.add_wagon = Some((v.id.clone(), VehicleKind::FreightCar));
                                    }
                                    if ui
                                        .add(
                                            egui::Button::new(RichText::new(if is_de { "+ Person" } else { "+ Coach" }).size(10.5).color(p.text_main))
                                                .fill(p.bg_sidebar)
                                                .stroke(Stroke::new(1.0_f32, p.border_main))
                                                .min_size(egui::vec2(62.0, 23.0))
                                                .corner_radius(CornerRadius::same(5)),
                                        )
                                        .clicked()
                                    {
                                        actions.add_wagon = Some((v.id.clone(), VehicleKind::PassengerCoach));
                                    }
                                });
                            });

                            if !v.coupled_cars.is_empty() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                                    for (ci, wk) in v.coupled_cars.iter().enumerate() {
                                        let short = match wk {
                                            VehicleKind::PassengerCoach => if is_de { "Pers." } else { "Coach" },
                                            VehicleKind::FreightCar => if is_de { "Gueter" } else { "Frt" },
                                            _ => "Car",
                                        };
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new(format!("#{} {} | x", ci + 1, short))
                                                        .strong()
                                                        .size(10.0)
                                                        .color(Color32::WHITE),
                                                )
                                                .fill(Color32::from_rgb(109, 40, 217))
                                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(139, 92, 246)))
                                                .min_size(egui::vec2(0.0, 22.0))
                                                .corner_radius(CornerRadius::same(5)),
                                            )
                                            .on_hover_text(if is_de {
                                                "Wagen ab dieser Stelle vom Zug abkuppeln (bleibt auf dem Gleis)"
                                            } else {
                                                "Disconnect from train at this coupler (leaves wagons on track)"
                                            })
                                            .clicked()
                                        {
                                            actions.uncouple_at = Some((v.id.clone(), ci));
                                        }
                                    }
                                });
                            }

                            ui.horizontal(|ui| {
                                if !v.coupled_cars.is_empty() {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(if is_de { "Letzten abk." } else { "Uncouple Last" })
                                                    .strong()
                                                    .size(10.5)
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(Color32::from_rgb(124, 58, 237))
                                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(167, 139, 250)))
                                            .min_size(egui::vec2(82.0, 23.0))
                                            .corner_radius(CornerRadius::same(5)),
                                        )
                                        .on_hover_text(if is_de { "Letzten Wagen abkuppeln (U)" } else { "Disconnect last wagon (U)" })
                                        .clicked()
                                    {
                                        actions.uncouple_at = Some((v.id.clone(), v.coupled_cars.len() - 1));
                                    }
                                    if v.coupled_cars.len() >= 2
                                        && ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new(if is_de { "Alle abk." } else { "Uncouple All" })
                                                        .strong()
                                                        .size(10.5)
                                                        .color(p.text_main),
                                                )
                                                .fill(p.bg_sidebar)
                                                .stroke(Stroke::new(1.0_f32, p.border_main))
                                                .min_size(egui::vec2(72.0, 23.0))
                                                .corner_radius(CornerRadius::same(5)),
                                            )
                                            .on_hover_text(if is_de { "Alle Wagen abkuppeln (Umschalt+U)" } else { "Disconnect all wagons (Shift+U)" })
                                            .clicked()
                                    {
                                        actions.uncouple_at = Some((v.id.clone(), 0));
                                    }
                                }
                                let c_fill = if has_nearby_couple {
                                    Color32::from_rgb(22, 163, 74)
                                } else {
                                    p.bg_sidebar
                                };
                                let c_txt = if has_nearby_couple { Color32::WHITE } else { p.text_muted };
                                if ui
                                    .add_enabled(
                                        has_nearby_couple,
                                        egui::Button::new(
                                            RichText::new(if is_de { "Ankuppeln" } else { "Couple" })
                                                .strong()
                                                .size(10.5)
                                                .color(c_txt),
                                        )
                                        .fill(c_fill)
                                        .stroke(Stroke::new(
                                            1.0_f32,
                                            if has_nearby_couple { Color32::from_rgb(74, 222, 128) } else { p.border_main },
                                        ))
                                        .min_size(egui::vec2(72.0, 23.0))
                                        .corner_radius(CornerRadius::same(5)),
                                    )
                                    .on_hover_text(if is_de {
                                        "Stehende Wagen hinter der Lok ankuppeln (C)"
                                    } else {
                                        "Couple parked wagons behind locomotive (C)"
                                    })
                                    .clicked()
                                {
                                    actions.couple_nearby_id = Some(v.id.clone());
                                }
                            });
                        }

                        ui.label(
                            RichText::new(if is_de {
                                "W/S: Regler  ·  R: Dir  ·  L: Licht  ·  U: Abk.  ·  C: Ank.  ·  Space: Halt"
                            } else {
                                "W/S: Throttle  ·  R: Dir  ·  L: Lights  ·  U: Uncouple  ·  C: Couple  ·  Space: Stop"
                            })
                            .size(9.5)
                            .color(p.text_muted),
                        );
                        }
                    });

                    if start_sim {
                        app.vehicle_sim.running = true;
                    }
                } else {
                    draw_desk_card(ui, p, |ui| {
                        ui.label(
                            RichText::new(if is_de {
                                "Kein Fahrzeug aktiv — unten eine Lok oder ein Strassenfahrzeug platzieren."
                            } else {
                                "No active vehicle — spawn a locomotive or road vehicle below."
                            })
                            .size(11.5)
                            .color(p.text_muted),
                        );
                    });
                }

                // 3. Spawn Rail & Road Vehicles Card
                let has_rail = app.layout.pieces.iter().any(|pc| {
                    crate::vehicle_sim::VehicleSimState::piece_matches_domain(
                        &app.catalogs,
                        pc,
                        VehicleDomain::Rail,
                    ) && pc.connectors.len() >= 2
                });
                let has_road = app.layout.pieces.iter().any(|pc| {
                    crate::vehicle_sim::VehicleSimState::piece_matches_domain(
                        &app.catalogs,
                        pc,
                        VehicleDomain::Road,
                    ) && pc.connectors.len() >= 2
                });
                draw_desk_card(ui, p, |ui| {
                    ui.label(
                        RichText::new(if is_de { "FAHRZEUG PLATZIEREN" } else { "SPAWN VEHICLE" })
                            .strong()
                            .size(10.5)
                            .color(p.text_muted),
                    );
                    ui.label(RichText::new(if is_de { "Schiene" } else { "Rail" }).size(10.5).color(p.text_sub));
                    ui.columns(3, |cols| {
                        for (ci, &(kind, lbl)) in [
                            (VehicleKind::Br243, "+ BR 243"),
                            (VehicleKind::Br96Mallet, "+ BR 96"),
                            (VehicleKind::V60Diesel, "+ V 60"),
                        ]
                        .iter()
                        .enumerate()
                        {
                            cols[ci].add_enabled_ui(has_rail, |ui| {
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 24.0],
                                        egui::Button::new(
                                            RichText::new(lbl)
                                                .strong()
                                                .size(11.0)
                                                .color(if has_rail { p.text_main } else { p.text_muted }),
                                        )
                                        .fill(p.bg_sidebar)
                                        .stroke(Stroke::new(1.0_f32, p.border_main))
                                        .corner_radius(CornerRadius::same(6)),
                                    )
                                    .on_disabled_hover_text(if is_de {
                                        "Zuerst mindestens ein Gleis auf der Anlage platzieren."
                                    } else {
                                        "Place at least one rail track piece first."
                                    })
                                    .clicked()
                                {
                                    actions.spawn = Some(kind);
                                }
                            });
                        }
                    });
                    ui.label(RichText::new(if is_de { "Strasse" } else { "Road" }).size(10.5).color(p.text_sub));
                    ui.columns(3, |cols| {
                        for (ci, &(kind, lbl_de, lbl_en)) in [
                            (VehicleKind::Sedan, "+ PKW", "+ Car"),
                            (VehicleKind::Truck, "+ LKW", "+ Truck"),
                            (VehicleKind::Bus, "+ Bus", "+ Bus"),
                        ]
                        .iter()
                        .enumerate()
                        {
                            cols[ci].add_enabled_ui(has_road, |ui| {
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 24.0],
                                        egui::Button::new(
                                            RichText::new(if is_de { lbl_de } else { lbl_en })
                                                .strong()
                                                .size(11.0)
                                                .color(if has_road { p.text_main } else { p.text_muted }),
                                        )
                                        .fill(p.bg_sidebar)
                                        .stroke(Stroke::new(1.0_f32, p.border_main))
                                        .corner_radius(CornerRadius::same(6)),
                                    )
                                    .on_disabled_hover_text(if is_de {
                                        "Zuerst mindestens eine Strasse aus dem Katalog platzieren."
                                    } else {
                                        "Place at least one street piece from the catalog first."
                                    })
                                    .clicked()
                                {
                                    actions.spawn = Some(kind);
                                }
                            });
                        }
                    });
                });

                // 4. Turnout Switcher Card
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
                    draw_desk_card(ui, p, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(if is_de {
                                    format!("WEICHEN ({})", turnouts.len())
                                } else {
                                    format!("TURNOUTS ({})", turnouts.len())
                                })
                                .strong()
                                .size(10.5)
                                .color(p.text_muted),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if !app.vehicle_sim.turnout_states.is_empty()
                                    && ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(if is_de { "Alle gerade" } else { "All Straight" })
                                                    .size(10.0)
                                                    .color(p.text_main),
                                            )
                                            .fill(p.bg_sidebar)
                                            .stroke(Stroke::new(1.0_f32, p.border_main))
                                            .min_size(egui::vec2(64.0, 21.0))
                                            .corner_radius(CornerRadius::same(5)),
                                        )
                                        .clicked()
                                {
                                    app.vehicle_sim.turnout_states.clear();
                                }
                            });
                        });
                        for row in turnouts.iter().take(18).collect::<Vec<_>>().chunks(3) {
                            ui.columns(3, |cols| {
                                for (ci, (tid, cid, br_cnt, is_dkw)) in row.iter().enumerate() {
                                    let st = app.vehicle_sim.turnout_states.get(tid).copied().unwrap_or(0) % (*br_cnt).max(2);
                                    let lbl = if *is_dkw {
                                        if st == 0 {
                                            format!("{cid} X")
                                        } else {
                                            format!("{cid} )(")
                                        }
                                    } else if st == 0 {
                                        format!("{cid} |")
                                    } else if *br_cnt >= 3 && st == 1 {
                                        format!("{cid} /L")
                                    } else if *br_cnt >= 3 && st == 2 {
                                        format!("{cid} \\R")
                                    } else {
                                        format!("{cid} /{st}")
                                    };
                                    let (fill, stroke_col, txt_col) = if st != 0 {
                                        (
                                            Color32::from_rgb(217, 119, 6),
                                            Color32::from_rgb(245, 158, 11),
                                            Color32::WHITE,
                                        )
                                    } else if app.dark_mode {
                                        (
                                            Color32::from_rgb(12, 74, 110),
                                            p.primary,
                                            Color32::WHITE,
                                        )
                                    } else {
                                        (p.bg_sidebar, p.primary, p.text_main)
                                    };
                                    let btn = egui::Button::new(RichText::new(lbl).strong().size(10.5).color(txt_col))
                                        .fill(fill)
                                        .stroke(Stroke::new(1.0_f32, stroke_col))
                                        .corner_radius(CornerRadius::same(5));
                                    if cols[ci].add_sized([cols[ci].available_width(), 24.0], btn).clicked() {
                                        actions.toggle_turnout = Some((tid.clone(), *br_cnt));
                                    }
                                }
                            });
                        }
                    });
                }

                // 5. Uncoupler Tracks Card (e.g. Maerklin 24997)
                super::viewport_and_controls::draw_uncoupler_controls(ui, app, p, is_de, actions);

                // 6. Turntable & Transfer Table Card
                super::viewport_and_controls::draw_turntable_transfer_controls(ui, app, p, is_de);
            });
        });
}

pub(super) fn draw_desk_card(
    ui: &mut egui::Ui,
    p: crate::theme::RailbenchPalette,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(9))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui);
        });
}

fn draw_telemetry_tile(
    ui: &mut egui::Ui,
    p: crate::theme::RailbenchPalette,
    label: &str,
    value: String,
    val_col: Color32,
) {
    Frame::new()
        .fill(p.bg_sidebar)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(6, 5))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(label).size(9.0).strong().color(p.text_muted));
                ui.label(RichText::new(value).strong().monospace().size(12.0).color(val_col));
            });
        });
}
