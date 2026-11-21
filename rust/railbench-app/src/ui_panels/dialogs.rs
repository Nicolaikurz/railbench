use super::*;

pub fn draw_dialogs(ctx: &egui::Context, app: &mut RailbenchApp) {
    let t = tr(app.lang);

    if app.show_power_dialog {
        let mut open = app.show_power_dialog;
        let is_de = app.lang == Lang::De;
        let sim_snapshot = app.power_sim_cache.as_ref().map(|(_, s)| std::sync::Arc::clone(s));
        let mut param_changed = false;
        let mut focus_pt: Option<(f64, f64)> = None;
        let mut bulk_equip: Option<bool> = None;

        let mut total_turnouts = 0usize;
        let mut lantern_count = 0usize;
        let mut decoder_count = 0usize;
        let mut cat_pieces = 0usize;
        let mut cat_feeders = 0usize;
        let mut cat_isolators = 0usize;
        for pc in &app.layout.pieces {
            if crate::catalog::find_entry(&app.catalogs, &pc.catalog_id)
                .map_or(false, |e| crate::is_turnout_kind(&e.kind))
            {
                total_turnouts += 1;
                if pc.has_lantern == Some(true) {
                    lantern_count += 1;
                }
                if pc.has_decoder == Some(true) {
                    decoder_count += 1;
                }
            }
            if pc.has_catenary == Some(true) {
                cat_pieces += 1;
            }
            if pc.catenary_power_feed == Some(true) {
                cat_feeders += 1;
            }
            if pc.catenary_isolator == Some(true) {
                cat_isolators += 1;
            }
        }

        egui::Window::new(t.power_dialog)
            .open(&mut open)
            .resizable(false)
            .default_width(390.0)
            .show(ctx, |ui| {
                ui.checkbox(
                    &mut app.power_sim_enabled,
                    if is_de {
                        "Stromanalyse auf 2D-Gleisplan einblenden"
                    } else {
                        "Show Power Overlay on 2D Canvas"
                    },
                );
                ui.separator();

                let mut v = app.layout.feed_voltage.unwrap_or(16.0);
                let mut ma = app.layout.loco_current_ma.unwrap_or(450.0);
                let mut cnt = app.layout.loco_count.unwrap_or(1);

                ui.horizontal(|ui| {
                    ui.label(if is_de { "Vorauswahl:" } else { "Presets:" });
                    for (lbl, pv, pma) in [
                        ("12V (N)", 12.0, 100.0),
                        ("14V (DCC)", 14.0, 450.0),
                        ("16V (H0)", 16.0, 450.0),
                        ("18V (1)", 18.0, 800.0),
                    ] {
                        if ui.small_button(lbl).clicked() {
                            app.layout.feed_voltage = Some(pv);
                            app.layout.loco_current_ma = Some(pma);
                            app.power_sim_enabled = true;
                            v = pv;
                            ma = pma;
                            param_changed = true;
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(if is_de { "Fahrspannung:" } else { "Feed Voltage:" });
                    if ui
                        .add(egui::DragValue::new(&mut v).range(6.0..=24.0).speed(0.5).suffix(" V"))
                        .changed()
                    {
                        app.layout.feed_voltage = Some(v);
                        param_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Lokstrom:" } else { "Loco Current:" });
                    if ui
                        .add(egui::DragValue::new(&mut ma).range(50.0..=1500.0).speed(10.0).suffix(" mA"))
                        .changed()
                    {
                        app.layout.loco_current_ma = Some(ma);
                        param_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Aktive Loks:" } else { "Locos running:" });
                    if ui.add(egui::DragValue::new(&mut cnt).range(1..=20)).changed() {
                        app.layout.loco_count = Some(cnt);
                        param_changed = true;
                    }
                });

                ui.separator();
                ui.label(
                    RichText::new(if is_de {
                        "Oberleitung (2. Stromkreis)"
                    } else {
                        "Catenary Circuit"
                    })
                    .strong(),
                );
                let is_parallel = app.layout.catenary_circuit_mode.as_deref() == Some("parallel");
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Betriebsart:" } else { "Mode:" });
                    if ui
                        .selectable_label(!is_parallel, if is_de { "Getrennt" } else { "Separate" })
                        .clicked()
                    {
                        app.layout.catenary_circuit_mode = Some("separate".into());
                        param_changed = true;
                    }
                    if ui
                        .selectable_label(is_parallel, if is_de { "Parallel zum Gleis" } else { "Parallel to Track" })
                        .clicked()
                    {
                        app.layout.catenary_circuit_mode = Some("parallel".into());
                        param_changed = true;
                    }
                });
                if !is_parallel {
                    let mut cat_v = app.layout.catenary_feed_voltage.unwrap_or(v);
                    ui.horizontal(|ui| {
                        ui.label(if is_de { "Oberleitungsspannung:" } else { "Catenary Voltage:" });
                        if ui
                            .add(egui::DragValue::new(&mut cat_v).range(6.0..=24.0).speed(0.5).suffix(" V"))
                            .changed()
                        {
                            app.layout.catenary_feed_voltage = Some(cat_v);
                            param_changed = true;
                        }
                    });
                }
                ui.small(if is_de {
                    format!("Oberleitung: {cat_pieces} Gleise | {cat_feeders} Einspeiser | {cat_isolators} Trennstellen")
                } else {
                    format!("Catenary: {cat_pieces} tracks | {cat_feeders} feeders | {cat_isolators} isolators")
                });

                ui.separator();
                ui.label(
                    RichText::new(if is_de {
                        "Weichenlaternen & Digital-Decoder"
                    } else {
                        "Turnout Lanterns & Decoders"
                    })
                    .strong(),
                );
                let mut l_ma = app.layout.turnout_lantern_current_ma.unwrap_or(20.0);
                let mut d_ma = app.layout.turnout_decoder_current_ma.unwrap_or(15.0);
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Laterne:" } else { "Lantern:" });
                    if ui
                        .add(egui::DragValue::new(&mut l_ma).range(0.0..=100.0).speed(1.0).suffix(" mA"))
                        .changed()
                    {
                        app.layout.turnout_lantern_current_ma = Some(l_ma);
                        param_changed = true;
                    }
                    ui.label(if is_de { "Decoder:" } else { "Decoder:" });
                    if ui
                        .add(egui::DragValue::new(&mut d_ma).range(0.0..=100.0).speed(1.0).suffix(" mA"))
                        .changed()
                    {
                        app.layout.turnout_decoder_current_ma = Some(d_ma);
                        param_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.small(if is_de {
                        format!("Weichen: {total_turnouts} | Laternen: {lantern_count} | Decoder: {decoder_count}")
                    } else {
                        format!("Turnouts: {total_turnouts} | Lanterns: {lantern_count} | Decoders: {decoder_count}")
                    });
                });
                ui.horizontal(|ui| {
                    if ui
                        .button(if is_de { "Alle Weichen ausstatten" } else { "Equip All Turnouts" })
                        .clicked()
                    {
                        bulk_equip = Some(true);
                    }
                    if ui
                        .button(if is_de { "Alle entfernen" } else { "Clear All" })
                        .clicked()
                    {
                        bulk_equip = Some(false);
                    }
                });

                if let Some(sim) = sim_snapshot {
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "System: {}  |  Load: {:.2} A",
                            if sim.is_two_wire { "2-Wire (DCC/DC)" } else { "3-Wire AC" },
                            sim.total_current_a
                        ))
                        .strong(),
                    );
                    ui.label(format!(
                        "Feeders: {}  |  Isolators: {}  |  Min Voltage: {:.2} V (Drop {:.1}%)",
                        sim.feeder_count, sim.isolator_count, sim.min_voltage, sim.max_drop_pct
                    ));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("OK {}", sim.optimal_count)).color(Color32::from_rgb(34, 197, 94)));
                        ui.label(RichText::new(format!("Warn {}", sim.warning_count)).color(Color32::from_rgb(245, 158, 11)));
                        ui.label(RichText::new(format!("Crit {}", sim.critical_count)).color(Color32::from_rgb(239, 68, 68)));
                        ui.label(RichText::new(format!("Dead {}", sim.disconnected_count)).color(Color32::from_rgb(148, 163, 184)));
                    });
                    if sim.has_short_circuit {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("SHORT CIRCUIT ({} point(s))", sim.short_circuit_points.len()))
                                    .strong()
                                    .color(Color32::from_rgb(239, 68, 68)),
                            );
                            if let Some(pt) = sim.short_circuit_points.first() {
                                if ui.small_button("Focus").clicked() {
                                    focus_pt = Some((pt.x, pt.y));
                                }
                            }
                        });
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new(&sim.recommendation).size(11.5));
                }
            });
        app.show_power_dialog = open;
        if let Some(equip) = bulk_equip {
            app.bulk_equip_turnouts(equip);
        } else if param_changed {
            app.mark_layout_dirty();
        }
        if let Some((x, y)) = focus_pt {
            app.focus_world_point(x, y);
        }
    }

    if app.show_height_dialog {
        let mut open = app.show_height_dialog;
        let nem_h = min_clearance_for_scale(&app.layout.scale);
        let scale_label = app.layout.scale.clone();
        egui::Window::new(t.height_dialog)
            .open(&mut open)
            .resizable(false)
            .default_width(350.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut app.height_tab, 0, "Flat Height");
                    ui.selectable_value(&mut app.height_tab, 1, "Auto-Ramp");
                    ui.selectable_value(&mut app.height_tab, 2, "Ramp / Helix Gen");
                });
                ui.separator();

                match app.height_tab {
                    0 => {
                        ui.horizontal(|ui| {
                            ui.label("Elevation:");
                            ui.add(egui::DragValue::new(&mut app.batch_elevation_mm).speed(2.0).suffix(" mm"));
                            if ui.small_button(format!("NEM 102 ({nem_h:.0} mm {scale_label})")).clicked() {
                                app.batch_elevation_mm = nem_h;
                            }
                        });
                        ui.add_space(4.0);
                        if ui
                            .add_enabled(!app.selected.is_empty(), egui::Button::new("Apply flat height to selected"))
                            .clicked()
                        {
                            app.push_undo();
                            let h = app.batch_elevation_mm;
                            for piece in &mut app.layout.pieces {
                                if app.selected.contains(&piece.id) {
                                    piece.elevation = Some(Elevation { start: h, end: h });
                                }
                            }
                            railbench_core::snapping::update_layout_connections(
                                &mut app.layout,
                                4.0,
                                35.0,
                                1.0,
                            );
                            crate::catalog::disconnect_cross_type_connections(
                                &mut app.layout,
                                &app.catalogs,
                            );
                            app.mark_layout_dirty();
                        }
                    }
                    1 => {
                        let is_de = app.lang == Lang::De;
                        let single_sel_id = if app.selected.len() == 1 {
                            app.selected.iter().next().cloned()
                        } else {
                            None
                        };
                        let piece_label = |pid: Option<&str>| -> String {
                            match pid {
                                Some(id) => app
                                    .layout
                                    .pieces
                                    .iter()
                                    .find(|pc| pc.id == id)
                                    .map(|pc| format!("{} ({})", pc.catalog_id, pc.id))
                                    .unwrap_or_else(|| id.to_string()),
                                None => if is_de { "Auto (Auswahl)".into() } else { "Auto (Selection)".into() },
                            }
                        };

                        // Start (A) & End (B) point selector
                        egui::Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(15, 23, 42, 48))
                            .corner_radius(6.0)
                            .inner_margin(egui::Margin::same(6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("A")
                                            .strong()
                                            .color(Color32::from_rgb(16, 185, 129)),
                                    );
                                    ui.label(if is_de { "Start:" } else { "Start:" });
                                    ui.label(RichText::new(piece_label(app.ramp_sel.start_piece.as_deref())).small());
                                    let picking_a = app.ramp_sel.pick_mode == Some(true);
                                    let pick_a_btn = egui::Button::new(if picking_a {
                                        if is_de { "Klicke Gleis…" } else { "Click track…" }
                                    } else {
                                        if is_de { "Wählen" } else { "Pick A" }
                                    })
                                    .fill(if picking_a {
                                        Color32::from_rgb(16, 185, 129)
                                    } else {
                                        ui.visuals().widgets.inactive.weak_bg_fill
                                    });
                                    if ui.add(pick_a_btn).clicked() {
                                        app.ramp_sel.pick_mode = if picking_a { None } else { Some(true) };
                                    }
                                    if let Some(ref sid) = single_sel_id {
                                        if ui.small_button(if is_de { "Auswahl" } else { "From Sel" }).clicked() {
                                            app.ramp_sel.start_piece = Some(sid.clone());
                                            app.ramp_sel.pick_mode = None;
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("B")
                                            .strong()
                                            .color(Color32::from_rgb(245, 158, 11)),
                                    );
                                    ui.label(if is_de { "Ende: " } else { "End:  " });
                                    ui.label(RichText::new(piece_label(app.ramp_sel.end_piece.as_deref())).small());
                                    let picking_b = app.ramp_sel.pick_mode == Some(false);
                                    let pick_b_btn = egui::Button::new(if picking_b {
                                        if is_de { "Klicke Gleis…" } else { "Click track…" }
                                    } else {
                                        if is_de { "Wählen" } else { "Pick B" }
                                    })
                                    .fill(if picking_b {
                                        Color32::from_rgb(245, 158, 11)
                                    } else {
                                        ui.visuals().widgets.inactive.weak_bg_fill
                                    });
                                    if ui.add(pick_b_btn).clicked() {
                                        app.ramp_sel.pick_mode = if picking_b { None } else { Some(false) };
                                    }
                                    if let Some(ref sid) = single_sel_id {
                                        if ui.small_button(if is_de { "Auswahl" } else { "From Sel" }).clicked() {
                                            app.ramp_sel.end_piece = Some(sid.clone());
                                            app.ramp_sel.pick_mode = None;
                                        }
                                    }
                                });
                            });

                        let chain = resolve_ramp_chain(
                            &app.layout,
                            &app.selected,
                            app.ramp_sel.start_piece.as_deref(),
                            app.ramp_sel.end_piece.as_deref(),
                        );

                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    chain.len() >= 2 || app.ramp_sel.start_piece.is_some() || app.ramp_sel.end_piece.is_some(),
                                    egui::Button::new(if is_de { "⇄ Richtung tauschen (A ↔ B)" } else { "⇄ Swap Direction (A ↔ B)" }),
                                )
                                .clicked()
                            {
                                if app.ramp_sel.start_piece.is_some() || app.ramp_sel.end_piece.is_some() {
                                    std::mem::swap(&mut app.ramp_sel.start_piece, &mut app.ramp_sel.end_piece);
                                } else if chain.len() >= 2 {
                                    app.ramp_sel.start_piece = chain.last().map(|(id, _)| id.clone());
                                    app.ramp_sel.end_piece = chain.first().map(|(id, _)| id.clone());
                                }
                            }
                            if (app.ramp_sel.start_piece.is_some() || app.ramp_sel.end_piece.is_some())
                                && ui.small_button(if is_de { "Zurücksetzen" } else { "Clear" }).clicked()
                            {
                                app.ramp_sel.start_piece = None;
                                app.ramp_sel.end_piece = None;
                                app.ramp_sel.pick_mode = None;
                            }
                        });

                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            ui.label("Start (A):");
                            ui.add(egui::DragValue::new(&mut app.ramp_start_h).speed(2.0).suffix(" mm"));
                            ui.label("End (B):");
                            ui.add(egui::DragValue::new(&mut app.ramp_end_h).speed(2.0).suffix(" mm"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Profile:");
                            ui.radio_value(&mut app.ramp_profile, RampProfile::Linear, "Linear");
                            ui.radio_value(&mut app.ramp_profile, RampProfile::Eased, "Eased (S-Curve)");
                        });

                        if let Some(sum) = preview_auto_ramp_for_chain(
                            &app.layout,
                            &app.catalogs,
                            &chain,
                            app.ramp_start_h,
                            app.ramp_end_h,
                            app.ramp_profile,
                        ) {
                            let grade_col = if sum.max_gradient_pct <= 3.0 {
                                Color32::from_rgb(34, 197, 94)
                            } else if sum.max_gradient_pct <= 4.0 {
                                Color32::from_rgb(245, 158, 11)
                            } else {
                                Color32::from_rgb(239, 68, 68)
                            };
                            let dir_text = format!(
                                "A ({:.0} mm)  →  B ({:.0} mm)  •  {} pieces",
                                app.ramp_start_h, app.ramp_end_h, sum.piece_count
                            );
                            ui.label(RichText::new(dir_text).small().color(Color32::from_rgb(56, 189, 248)));
                            ui.label(format!(
                                "Length: {:.1} mm  •  Rise: {:.1} mm",
                                sum.total_length_mm, sum.total_rise_mm
                            ));
                            ui.label(
                                RichText::new(format!("Peak grade: {:.1}% (avg {:.1}%)", sum.max_gradient_pct, sum.avg_gradient_pct))
                                    .strong()
                                    .color(grade_col),
                            );
                        } else if app.ramp_sel.start_piece.is_some() && app.ramp_sel.end_piece.is_some() {
                            ui.label(
                                RichText::new(if is_de {
                                    "Kein verbundener Gleisweg zwischen A und B gefunden."
                                } else {
                                    "No connected track path found between A and B."
                                })
                                .color(Color32::from_rgb(239, 68, 68)),
                            );
                        } else {
                            ui.weak(if is_de {
                                "Startpunkt (A) & Endpunkt (B) wählen oder Gleiskette markieren."
                            } else {
                                "Pick Start (A) & End (B) tracks on the canvas, or select a track chain."
                            });
                        }
                        ui.add_space(4.0);
                        if ui
                            .add_enabled(!chain.is_empty(), egui::Button::new("Apply Auto-Ramp"))
                            .clicked()
                        {
                            app.push_undo();
                            match apply_auto_ramp_for_chain(
                                &mut app.layout,
                                &app.catalogs,
                                &chain,
                                app.ramp_start_h,
                                app.ramp_end_h,
                                app.ramp_profile,
                            ) {
                                Ok(sum) => {
                                    app.ramp_sel.pick_mode = None;
                                    app.mark_layout_dirty();
                                    app.status_message = Some(format!(
                                        "Applied ramp over {} pieces ({:.1}% peak grade)",
                                        sum.piece_count, sum.max_gradient_pct
                                    ));
                                }
                                Err(e) => app.status_message = Some(e),
                            }
                        }
                    }
                    _ => {
                        let single_sel = if app.selected.len() == 1 {
                            app.selected.iter().next().cloned()
                        } else {
                            None
                        };
                        if single_sel.is_none() {
                            ui.label(
                                RichText::new("Select 1 anchor track with an open connector to generate from.")
                                    .color(Color32::from_rgb(245, 158, 11)),
                            );
                        }
                        ui.horizontal(|ui| {
                            ui.label("Shape:");
                            ui.radio_value(&mut app.gen_ramp_shape, RampShape::Straight, "Straight");
                            ui.radio_value(&mut app.gen_ramp_shape, RampShape::Curved, "Curved");
                            ui.radio_value(&mut app.gen_ramp_shape, RampShape::Helix, "Helix");
                        });
                        if app.gen_ramp_shape != RampShape::Straight {
                            ui.horizontal(|ui| {
                                ui.label("Direction:");
                                ui.radio_value(&mut app.gen_curve_left, true, "Left");
                                ui.radio_value(&mut app.gen_curve_left, false, "Right");
                            });
                            if let Some(cat) = app.catalogs.get(app.selected_catalog_idx) {
                                let curves = available_curves_for_catalog(cat);
                                if !curves.is_empty() {
                                    let sel_label = app
                                        .gen_curve_id
                                        .as_deref()
                                        .and_then(|cid| curves.iter().find(|c| c.id == cid))
                                        .map(|c| format!("{} (R{:.0})", c.id, c.radius.unwrap_or(0.0)))
                                        .unwrap_or_else(|| "Auto standard curve".into());
                                    egui::ComboBox::from_id_salt("gen_curve_combo")
                                        .selected_text(sel_label)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut app.gen_curve_id, None, "Auto standard curve");
                                            for c in &curves {
                                                let lbl = format!(
                                                    "{} • R{:.0} mm ({:.1}°)",
                                                    c.id,
                                                    c.radius.unwrap_or(0.0),
                                                    c.angle.unwrap_or(30.0)
                                                );
                                                ui.selectable_value(&mut app.gen_curve_id, Some(c.id.clone()), lbl);
                                            }
                                        });
                                }
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label("Target H:");
                            ui.add(egui::DragValue::new(&mut app.gen_target_h).speed(2.0).suffix(" mm"));
                            if ui.small_button(format!("+{nem_h:.0}")).clicked() {
                                app.gen_target_h = nem_h;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Max Grade:");
                            ui.add(egui::DragValue::new(&mut app.gen_max_grade).range(0.5..=8.0).speed(0.1).suffix(" %"));
                            if app.gen_ramp_shape == RampShape::Helix {
                                ui.label("Turns:");
                                ui.add(egui::DragValue::new(&mut app.gen_helix_turns).range(0.5..=6.0).speed(0.25));
                            }
                        });
                        ui.add_space(4.0);
                        if ui
                            .add_enabled(single_sel.is_some(), egui::Button::new("Generate Ramp / Helix"))
                            .clicked()
                        {
                            if let Some(anchor_id) = single_sel {
                                app.push_undo();
                                match generate_ramp_or_helix(
                                    &mut app.layout,
                                    &app.catalogs,
                                    app.selected_catalog_idx,
                                    &mut app.next_piece_seq,
                                    &anchor_id,
                                    app.gen_ramp_shape,
                                    app.gen_curve_left,
                                    app.gen_target_h,
                                    app.gen_max_grade,
                                    app.gen_helix_turns,
                                    app.gen_curve_id.as_deref(),
                                ) {
                                    Ok((new_ids, sum)) => {
                                        app.selected.clear();
                                        for id in new_ids {
                                            app.selected.insert(id);
                                        }
                                        app.mark_layout_dirty();
                                        app.status_message = Some(format!(
                                            "Generated {} pieces ({:.1}% grade, {:.1} mm rise)",
                                            sum.piece_count, sum.avg_gradient_pct, sum.total_rise_mm
                                        ));
                                    }
                                    Err(e) => app.status_message = Some(e),
                                }
                            }
                        }
                    }
                }
            });
        app.show_height_dialog = open;
    }

    if app.show_collab_dialog {
        let mut open = app.show_collab_dialog;
        egui::Window::new("Live Collaboration (WebSocket)")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Server:");
                    ui.text_edit_singleline(&mut app.collab.server_url);
                });
                ui.horizontal(|ui| {
                    ui.label("Room:");
                    ui.text_edit_singleline(&mut app.collab.room_id);
                });
                ui.horizontal(|ui| {
                    ui.label("User:");
                    ui.text_edit_singleline(&mut app.collab.user_name);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if app.collab.connected {
                        if ui.button("Disconnect").clicked() {
                            app.collab.disconnect();
                        }
                    } else if ui.button("Connect").clicked() {
                        app.collab.connect();
                    }
                    ui.weak(&app.collab.status);
                });
            });
        app.show_collab_dialog = open;
    }

    if app.show_welcome_dialog {
        draw_welcome_dialog(ctx, app);
    }

    if app.show_demo_dialog {
        draw_demo_dialog(ctx, app);
    }

    if app.show_settings_dialog {
        draw_settings_dialog(ctx, app);
    }

    if app.show_export_dialog {
        draw_export_dialog(ctx, app);
    }

    if app.show_help_dialog {
        draw_help_dialog(ctx, app);
    }
}

