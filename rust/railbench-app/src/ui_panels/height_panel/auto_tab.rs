use super::*;

#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub(super) fn draw_auto_tab(
    ui: &mut egui::Ui,
    app: &mut RailbenchApp,
    p: RailbenchPalette,
    is_de: bool,
    nem_h: f64,
    scale_label: &str,
    layers_list: &[(String, String, f64)],
    auto_apply_params: &mut Option<(String, usize)>,
) {
        // TAB 2: AUTOMATIC RAMP / CURVE / HELIX GENERATOR
        let options = collect_open_connector_options(
            &app.layout,
            &app.selected,
            app.gen_anchor_piece_id.as_deref(),
        );
        let mut current_opt = app.gen_anchor_piece_id.as_deref().and_then(|aid| {
            options.iter().find(|o| {
                o.piece_id == aid
                    && app
                        .gen_conn_idx
                        .is_none_or(|ci| o.connector_index == ci)
            })
        });
        if current_opt.is_none() {
            if let Some(first) = options.first() {
                app.gen_anchor_piece_id = Some(first.piece_id.clone());
                app.gen_conn_idx = Some(first.connector_index);
                current_opt = Some(first);
            } else {
                app.gen_anchor_piece_id = None;
                app.gen_conn_idx = None;
            }
        }
        let start_z = current_opt.map(|o| o.world_z).unwrap_or(0.0);

        // Start Track & Connector + compact Pick in layout button
        ui.horizontal(|ui| {
            section_label(
                ui,
                p,
                if is_de {
                    "Start-Gleis & Anschluss"
                } else {
                    "Start track & connector"
                },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let pick_lbl = if app.gen_pick_mode {
                    if is_de { "Klicke Gleis..." } else { "Click track..." }
                } else if is_de {
                    "Im Plan wählen"
                } else {
                    "Pick in layout"
                };
                let btn_w = if app.gen_pick_mode { 98.0 } else { 102.0 };
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(btn_w, 20.0), Sense::click());
                let bg = if app.gen_pick_mode {
                    Color32::from_rgba_unmultiplied(56, 189, 248, 45)
                } else if resp.hovered() {
                    Color32::from_rgba_unmultiplied(p.primary.r(), p.primary.g(), p.primary.b(), 28)
                } else {
                    Color32::TRANSPARENT
                };
                let stroke_c = if app.gen_pick_mode {
                    Color32::from_rgb(56, 189, 248)
                } else if resp.hovered() {
                    p.primary
                } else {
                    p.border_main
                };
                ui.painter().rect(
                    rect,
                    CornerRadius::same(5),
                    bg,
                    Stroke::new(1.0_f32, stroke_c),
                    egui::StrokeKind::Inside,
                );
                paint_pin_icon(
                    ui.painter(),
                    Pos2::new(rect.min.x + 10.0, rect.center().y),
                    4.4,
                    p.primary,
                );
                ui.painter().text(
                    Pos2::new(rect.min.x + 18.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    pick_lbl,
                    egui::FontId::proportional(10.5),
                    p.primary,
                );
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    app.gen_pick_mode = !app.gen_pick_mode;
                }
            });
        });

        let cur_conn_text = current_opt
            .map(|o| {
                format!(
                    "#{} · {} · {:.0} mm · ({:.0}, {:.0})",
                    o.catalog_id,
                    format_connector_name(o.connector_index, is_de),
                    o.world_z,
                    o.world_x,
                    o.world_y
                )
            })
            .unwrap_or_else(|| {
                if is_de {
                    "Keine freien Gleisanschlüsse gefunden".into()
                } else {
                    "No open connectors found".into()
                }
            });

        let mut chosen_key = current_opt
            .map(|o| format!("{}:{}", o.piece_id, o.connector_index))
            .unwrap_or_default();
        let prev_key = chosen_key.clone();
        egui::ComboBox::from_id_salt("hp_auto_start_conn_cb")
            .width(ui.available_width())
            .selected_text(cur_conn_text)
            .show_ui(ui, |ui| {
                for o in &options {
                    let key = format!("{}:{}", o.piece_id, o.connector_index);
                    let lbl = format!(
                        "#{} · {} · {:.0} mm · ({:.0}, {:.0})",
                        o.catalog_id,
                        format_connector_name(o.connector_index, is_de),
                        o.world_z,
                        o.world_x,
                        o.world_y
                    );
                    ui.selectable_value(&mut chosen_key, key, lbl);
                }
            });
        if chosen_key != prev_key {
            if let Some((pid, ci_str)) = chosen_key.split_once(':') {
                app.gen_anchor_piece_id = Some(pid.to_string());
                app.gen_conn_idx = ci_str.parse().ok();
            }
        }

        if let Some(o) = current_opt {
            ui.label(
                RichText::new(if is_de {
                    format!(
                        "Starthöhe: {:.0} mm · Richtung: {:.0}°",
                        o.world_z, o.angle_deg
                    )
                } else {
                    format!(
                        "Start height: {:.0} mm · Heading: {:.0}°",
                        o.world_z, o.angle_deg
                    )
                })
                .size(10.5)
                .color(p.text_muted),
            );
        }

        // Target Level (existing layers + create above/below)
        section_label(ui, p, if is_de { "Zielebene" } else { "Target level" });
        let above_lbl = if is_de {
            format!("+ Neue Ebene darüber (+{nem_h:.0} mm)")
        } else {
            format!("+ New layer above (+{nem_h:.0} mm)")
        };
        let below_lbl = if is_de {
            format!("+ Neue Ebene darunter (-{nem_h:.0} mm)")
        } else {
            format!("+ New layer below (-{nem_h:.0} mm)")
        };
        let layer_sel_text = if app.gen_target_layer_choice == LAYER_ABOVE_SENTINEL {
            above_lbl.clone()
        } else if app.gen_target_layer_choice == LAYER_BELOW_SENTINEL {
            below_lbl.clone()
        } else {
            layers_list
                .iter()
                .find(|(lid, _, _)| *lid == app.gen_target_layer_choice)
                .map(|(_, n, lh)| format!("{n} ({lh:.0} mm)"))
                .unwrap_or_else(|| above_lbl.clone())
        };
        let prev_layer_choice = app.gen_target_layer_choice.clone();
        egui::ComboBox::from_id_salt("hp_auto_target_layer_cb")
            .width(ui.available_width())
            .selected_text(layer_sel_text)
            .show_ui(ui, |ui| {
                for (lid, lname, lh) in layers_list {
                    ui.selectable_value(
                        &mut app.gen_target_layer_choice,
                        lid.clone(),
                        format!("{lname} ({lh:.0} mm)"),
                    );
                }
                ui.separator();
                ui.selectable_value(
                    &mut app.gen_target_layer_choice,
                    LAYER_ABOVE_SENTINEL.into(),
                    &above_lbl,
                );
                ui.selectable_value(
                    &mut app.gen_target_layer_choice,
                    LAYER_BELOW_SENTINEL.into(),
                    &below_lbl,
                );
            });
        if app.gen_target_layer_choice != prev_layer_choice {
            if app.gen_target_layer_choice == LAYER_ABOVE_SENTINEL {
                app.gen_target_layer_id = None;
                app.gen_slope_up = true;
                app.gen_target_h = start_z + nem_h;
            } else if app.gen_target_layer_choice == LAYER_BELOW_SENTINEL {
                app.gen_target_layer_id = None;
                app.gen_slope_up = false;
                app.gen_target_h = start_z - nem_h;
            } else if let Some((_, _, lh)) = layers_list
                .iter()
                .find(|(lid, _, _)| *lid == app.gen_target_layer_choice)
            {
                app.gen_target_layer_id = Some(app.gen_target_layer_choice.clone());
                app.gen_target_h = *lh;
                app.gen_slope_up = *lh >= start_z;
            }
        }

        // Target Height + Slope Up / Slope Down Pills
        ui.horizontal(|ui| {
            section_label(
                ui,
                p,
                if is_de { "Zielhöhe" } else { "Target height" },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("(Min. {nem_h:.0} mm)"))
                        .size(10.5)
                        .color(p.text_muted),
                );
            });
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let input_w = (ui.available_width() - 166.0).max(88.0);
            let dv = ui.add_sized(
                [input_w, 25.0],
                egui::DragValue::new(&mut app.gen_target_h)
                    .speed(1.0)
                    .fixed_decimals(0)
                    .suffix(" mm"),
            );
            if dv.changed() {
                app.gen_slope_up = app.gen_target_h >= start_z;
            }

            let dir_btn = |ui: &mut egui::Ui, up: bool, selected: bool, label: &str| {
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(78.0, 25.0), Sense::click());
                let active_bg = if app.dark_mode {
                    Color32::from_rgb(2, 132, 199)
                } else {
                    p.primary
                };
                let bg = if selected {
                    active_bg
                } else if resp.hovered() {
                    p.bg_card_hover
                } else {
                    p.bg_input
                };
                let border = if selected {
                    Stroke::new(1.0_f32, active_bg)
                } else {
                    Stroke::new(1.0_f32, p.border_main)
                };
                let fg = if selected { Color32::WHITE } else { p.text_sub };
                ui.painter().rect(
                    rect,
                    CornerRadius::same(6),
                    bg,
                    border,
                    egui::StrokeKind::Inside,
                );
                paint_trend_arrow_icon(
                    ui.painter(),
                    Pos2::new(rect.min.x + 12.0, rect.center().y),
                    up,
                    fg,
                );
                ui.painter().text(
                    Pos2::new(rect.min.x + 22.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(11.0),
                    fg,
                );
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                resp
            };

            let is_up = app.gen_target_h >= start_z;
            if dir_btn(
                ui,
                true,
                is_up,
                if is_de { "Steigung" } else { "Slope up" },
            )
            .clicked()
            {
                let delta = (app.gen_target_h - start_z).abs().max(nem_h);
                app.gen_slope_up = true;
                app.gen_target_h = start_z + delta;
            }
            if dir_btn(
                ui,
                false,
                !is_up,
                if is_de { "Gefälle" } else { "Slope down" },
            )
            .clicked()
            {
                let delta = (app.gen_target_h - start_z).abs().max(nem_h);
                app.gen_slope_up = false;
                app.gen_target_h = start_z - delta;
            }
        });

        // Ramp Shape 3-way segmented bar
        section_label(
            ui,
            p,
            if is_de {
                "Form der Rampe"
            } else {
                "Ramp shape"
            },
        );
        let shapes = [
            (RampShape::Straight, if is_de { "Gerade" } else { "Straight" }),
            (RampShape::Curved, if is_de { "Bogen" } else { "Curve" }),
            (RampShape::Helix, if is_de { "Wendel" } else { "Helix" }),
        ];
        segmented_control(
            ui,
            p,
            app.dark_mode,
            &mut app.gen_ramp_shape,
            &shapes,
            25.0,
            11.2,
        );

        // Curve / Helix Options
        if app.gen_ramp_shape != RampShape::Straight {
            let anchor_cat = current_opt
                .and_then(|o| {
                    app.catalogs
                        .iter()
                        .find(|c| c.entries.iter().any(|e| e.id == o.catalog_id))
                })
                .or_else(|| app.catalogs.get(app.selected_catalog_idx));
            let curves: Vec<CatalogEntry> = anchor_cat
                .map(available_curve_radii_dedup)
                .unwrap_or_default();
            if !curves.is_empty() {
                let chosen_curve = app
                    .gen_curve_id
                    .as_deref()
                    .and_then(|cid| curves.iter().find(|c| c.id == cid))
                    .or_else(|| curves.first());
                ui.horizontal(|ui| {
                    section_label(
                        ui,
                        p,
                        if is_de {
                            "Kurvenradius / Gleis"
                        } else {
                            "Curve radius / track"
                        },
                    );
                    if app.gen_ramp_shape == RampShape::Helix {
                        if let Some(rc) = chosen_curve.and_then(|c| c.radius) {
                            let circ = 2.0 * std::f64::consts::PI * rc;
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(if is_de {
                                            format!("Umfang: {circ:.0} mm")
                                        } else {
                                            format!("Circumference: {circ:.0} mm")
                                        })
                                        .size(10.5)
                                        .color(p.text_muted),
                                    );
                                },
                            );
                        }
                    }
                });

                let sel_curve_lbl = chosen_curve
                    .map(|c| {
                        format!(
                            "#{} (R{:.0} mm)",
                            c.id,
                            c.radius.unwrap_or(0.0)
                        )
                    })
                    .unwrap_or_else(|| {
                        if is_de {
                            "Standard-Radius".into()
                        } else {
                            "Standard radius".into()
                        }
                    });
                egui::ComboBox::from_id_salt("hp_auto_curve_cb")
                    .width(ui.available_width())
                    .selected_text(sel_curve_lbl)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.gen_curve_id,
                            None,
                            if is_de {
                                "Standard-Radius"
                            } else {
                                "Auto standard curve"
                            },
                        );
                        for (ri, c) in curves.iter().enumerate() {
                            let lbl = format!(
                                "R{} · #{} ({:.0} mm)",
                                ri + 1,
                                c.id,
                                c.radius.unwrap_or(0.0)
                            );
                            ui.selectable_value(
                                &mut app.gen_curve_id,
                                Some(c.id.clone()),
                                lbl,
                            );
                        }
                    });
            }

            section_label(
                ui,
                p,
                if is_de {
                    "Kurvenrichtung"
                } else {
                    "Curve direction"
                },
            );
            let dirs = [
                (true, if is_de { "Links" } else { "Left" }),
                (false, if is_de { "Rechts" } else { "Right" }),
            ];
            segmented_control(
                ui,
                p,
                app.dark_mode,
                &mut app.gen_curve_left,
                &dirs,
                24.0,
                11.2,
            );

            if app.gen_ramp_shape == RampShape::Curved {
                section_label(ui, p, if is_de { "Winkel" } else { "Angle" });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    let pill_w = ((ui.available_width() - 15.0) / 4.0).max(52.0);
                    for deg in [90.0_f64, 180.0, 270.0, 360.0] {
                        let active = (app.gen_arc_angle_deg - deg).abs() < 1.0;
                        if pill_button(
                            ui,
                            p,
                            app.dark_mode,
                            active,
                            pill_w,
                            25.0,
                            &format!("{deg:.0}°"),
                        )
                        .clicked()
                        {
                            app.gen_arc_angle_deg = deg;
                        }
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    section_label(
                        ui,
                        p,
                        if is_de { "Windungen" } else { "Turns" },
                    );
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            let mut manual = app.gen_helix_turns > 0.0;
                            if pill_button(
                                ui,
                                p,
                                app.dark_mode,
                                !manual,
                                48.0,
                                23.0,
                                "Auto",
                            )
                            .clicked()
                            {
                                app.gen_helix_turns = 0.0;
                                manual = false;
                            }
                            if manual {
                                ui.add_sized(
                                    [64.0, 23.0],
                                    egui::DragValue::new(&mut app.gen_helix_turns)
                                        .range(0.5..=6.0)
                                        .speed(0.25)
                                        .fixed_decimals(1),
                                );
                            } else if pill_button(
                                ui,
                                p,
                                app.dark_mode,
                                false,
                                64.0,
                                23.0,
                                if is_de { "Manuell" } else { "Manual" },
                            )
                            .clicked()
                            {
                                app.gen_helix_turns = 1.5;
                            }
                        },
                    );
                });
            }
        }

        // Target Slope Presets + Custom %
        section_label(
            ui,
            p,
            if is_de {
                "Ziel-Steigung"
            } else {
                "Target slope"
            },
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let pill_w = ((ui.available_width() - 74.0) / 4.0).max(46.0);
            for preset in [2.0_f64, 2.8, 3.5, 4.0] {
                let active = (app.gen_max_grade - preset).abs() < 0.05;
                if pill_button(
                    ui,
                    p,
                    app.dark_mode,
                    active,
                    pill_w,
                    25.0,
                    &format!("{preset:.1}%"),
                )
                .clicked()
                {
                    app.gen_max_grade = preset;
                }
            }
            ui.add_sized(
                [56.0, 25.0],
                egui::DragValue::new(&mut app.gen_max_grade)
                    .range(0.5..=8.0)
                    .speed(0.1)
                    .fixed_decimals(1)
                    .suffix(" %"),
            );
        });

        // Live Preview Computation & Colored Status Box
        if let Some(opt) = current_opt {
            let preview_layer_id = if app.gen_target_layer_choice == LAYER_ABOVE_SENTINEL
                || app.gen_target_layer_choice == LAYER_BELOW_SENTINEL
                || app.gen_target_layer_choice.is_empty()
            {
                None
            } else {
                Some(app.gen_target_layer_choice.as_str())
            };
            match preview_generate_ramp_or_helix(
                &app.layout,
                &app.catalogs,
                app.selected_catalog_idx,
                &opt.piece_id,
                Some(opt.connector_index),
                preview_layer_id,
                app.gen_ramp_shape,
                app.gen_curve_left,
                app.gen_arc_angle_deg,
                app.gen_target_h,
                app.gen_max_grade,
                app.gen_helix_turns,
                app.gen_curve_id.as_deref(),
            ) {
                Ok((preview_pieces, sum, actual_turns)) => {
                    app.gen_preview_pieces = preview_pieces;
                    *auto_apply_params =
                        Some((opt.piece_id.clone(), opt.connector_index));

                    let g = sum.avg_gradient_pct;
                    let kind = if g > 4.5 + 0.05 {
                        StatusKind::Danger
                    } else if g > 3.0 + 0.05 {
                        StatusKind::Warn
                    } else {
                        StatusKind::Ok
                    };
                    let turns_part = if app.gen_ramp_shape == RampShape::Helix {
                        if is_de {
                            format!(", {:.1} Wind.", actual_turns)
                        } else {
                            format!(", {:.1} turns", actual_turns)
                        }
                    } else {
                        String::new()
                    };
                    let prefix = match (kind, is_de) {
                        (StatusKind::Ok, true) => "Sichere Steigung",
                        (StatusKind::Ok, false) => "Safe gradient",
                        (StatusKind::Warn, true) => "Mittlere Steigung",
                        (StatusKind::Warn, false) => "Moderate gradient",
                        (StatusKind::Danger, true) => "Steile Steigung",
                        (StatusKind::Danger, false) => "Steep gradient",
                    };
                    let title = if is_de {
                        format!(
                            "{prefix}: {g:.1}% ({} Gleise{turns_part}, {:.1} cm)",
                            sum.piece_count,
                            sum.total_length_mm / 10.0
                        )
                    } else {
                        format!(
                            "{prefix}: {g:.1}% ({} tracks{turns_part}, {:.1} cm)",
                            sum.piece_count,
                            sum.total_length_mm / 10.0
                        )
                    };
                    let desc = format!(
                        "{:.0} mm -> {:.0} mm (Δ {:+.0} mm)",
                        opt.world_z, app.gen_target_h, sum.total_rise_mm
                    );
                    let extra_warn = if app.gen_ramp_shape == RampShape::Helix
                        && sum.total_rise_mm.abs() + 0.1 < nem_h
                    {
                        Some(if is_de {
                            format!(
                                "Hinweis: Durchfahrtshöhe unter NEM-Minimum ({nem_h:.0} mm für {scale_label})."
                            )
                        } else {
                            format!(
                                "Note: Clearance below NEM minimum ({nem_h:.0} mm for {scale_label})."
                            )
                        })
                    } else {
                        None
                    };
                    draw_status_card(
                        ui,
                        app.dark_mode,
                        kind,
                        &title,
                        &desc,
                        extra_warn.as_deref(),
                    );
                }
                Err(err_msg) => {
                    app.gen_preview_pieces.clear();
                    draw_status_card(
                        ui,
                        app.dark_mode,
                        StatusKind::Warn,
                        if is_de {
                            "Vorschau nicht möglich"
                        } else {
                            "Cannot preview ramp"
                        },
                        &err_msg,
                        None,
                    );
                }
            }
        } else {
            app.gen_preview_pieces.clear();
            draw_status_card(
                ui,
                app.dark_mode,
                StatusKind::Warn,
                if is_de {
                    "Kein Startgleis gewählt"
                } else {
                    "No start track selected"
                },
                if is_de {
                    "Wähle oben ein Startgleis oder klicke 'Im Plan wählen'."
                } else {
                    "Choose an open start connector above or click 'Pick in layout'."
                },
                None,
            );
        }
}
