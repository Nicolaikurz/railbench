use super::*;

pub fn draw_dialogs(ctx: &egui::Context, app: &mut RailbenchApp) {
    let t = tr(app.lang);

    if app.show_power_dialog {
        if app.power_draft.is_none() {
            app.power_draft = Some(crate::PowerDraft {
                feed_voltage: app.layout.feed_voltage,
                catenary_feed_voltage: app.layout.catenary_feed_voltage,
                catenary_circuit_mode: app.layout.catenary_circuit_mode.clone(),
                loco_current_ma: app.layout.loco_current_ma,
                loco_count: app.layout.loco_count,
                turnout_lantern_current_ma: app.layout.turnout_lantern_current_ma,
                turnout_decoder_current_ma: app.layout.turnout_decoder_current_ma,
                power_sim_enabled: app.power_sim_enabled,
            });
        }
        let mut open = app.show_power_dialog;
        let is_de = app.lang == Lang::De;
        let sim_snapshot = app.power_sim_cache.as_ref().map(|(_, s)| std::sync::Arc::clone(s));
        let mut param_changed = false;
        let mut focus_pt: Option<(f64, f64)> = None;
        let mut bulk_equip: Option<bool> = None;
        let mut do_add_feeder = false;
        let mut do_add_isolator = false;
        let mut do_cycle_iso_end = false;
        let mut cancel_revert = false;
        let mut apply_close = false;

        let sel_has_iso = app.layout.pieces.iter().any(|pc| {
            app.selected.contains(&pc.id)
                && !crate::catalog::is_road_id(&app.catalogs, &pc.catalog_id)
                && pc.is_isolator == Some(true)
        });

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
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(if is_de { "+ ⚡ Anschlussgleis" } else { "+ ⚡ Feeder Track" })
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(217, 119, 6)),
                        )
                        .clicked()
                    {
                        do_add_feeder = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(if is_de { "+ ✂ Trenngleis" } else { "+ ✂ Isolating Track" })
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(220, 38, 38)),
                        )
                        .clicked()
                    {
                        do_add_isolator = true;
                    }
                    if sel_has_iso
                        && ui
                            .small_button(if is_de { "⇄ Ende" } else { "⇄ End" })
                            .on_hover_text(if is_de { "Isoliertes Gleisende wechseln" } else { "Cycle isolated connector end" })
                            .clicked()
                    {
                        do_cycle_iso_end = true;
                    }
                });
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
                    for (plbl, pma) in [("100", 100.0), ("300", 300.0), ("450", 450.0), ("800", 800.0)] {
                        if ui.small_button(plbl).clicked() {
                            app.layout.loco_current_ma = Some(pma);
                            param_changed = true;
                        }
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
                        for pv in [12.0, 14.0, 16.0, 18.0, 24.0] {
                            if ui.small_button(format!("{pv:.0}V")).clicked() {
                                app.layout.catenary_feed_voltage = Some(pv);
                                param_changed = true;
                            }
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
                            "System: {}  |  Total: {:.2} A (Turnouts: {:.2} A)",
                            if sim.is_two_wire { "2-Wire (DCC/DC)" } else { "3-Wire AC" },
                            sim.total_current_a,
                            sim.turnout_current_a
                        ))
                        .strong(),
                    );
                    if sim.total_current_a > 3.0 {
                        ui.label(
                            RichText::new(if is_de {
                                "⚠ Booster-Ueberlast (>3.0 A)! Zusatzeinspeisung / Booster einplanen."
                            } else {
                                "⚠ Booster Overload Risk (>3.0 A)! Consider a second booster district."
                            })
                            .strong()
                            .size(11.5)
                            .color(Color32::from_rgb(245, 158, 11)),
                        );
                    }
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
                    if !sim.catenary_recommendation.is_empty() {
                        ui.small(&sim.catenary_recommendation);
                    }
                    if sim.has_short_circuit {
                        ui.add_space(6.0);
                        egui::Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(153, 27, 27, 220))
                            .stroke(egui::Stroke::new(1.5, Color32::from_rgb(252, 165, 165)))
                            .corner_radius(6.0)
                            .inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let hdr = if sim.is_two_wire {
                                        if is_de {
                                            format!("⚠ 2-LEITER KURZSCHLUSS ({} Stelle(n))", sim.short_circuit_points.len().max(1))
                                        } else {
                                            format!("⚠ 2-WIRE SHORT CIRCUIT ({} point(s))", sim.short_circuit_points.len().max(1))
                                        }
                                    } else {
                                        format!("⚠ SHORT CIRCUIT ({} point(s))", sim.short_circuit_points.len().max(1))
                                    };
                                    ui.label(RichText::new(hdr).strong().color(Color32::WHITE));
                                    if let Some(pt) = sim.short_circuit_points.first() {
                                        if ui.small_button(if is_de { "Fokus" } else { "Focus" }).clicked() {
                                            focus_pt = Some((pt.x, pt.y));
                                        }
                                    }
                                });
                                if sim.is_two_wire {
                                    ui.label(
                                        RichText::new(if is_de {
                                            "Gegenpolige Schienen sind verbunden (z. B. Kehrschleife / Gleisdreieck). Setze Trennstellen (Isolator) auf den betroffenen Gleisen."
                                        } else {
                                            "Opposite rail polarities are connected (e.g. reversing loop / wye). Place Isolator tracks to break the short."
                                        })
                                        .size(11.0)
                                        .color(Color32::from_rgb(254, 226, 226)),
                                    );
                                }
                            });
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new(&sim.recommendation).size(11.5));
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(if is_de { "Abbrechen" } else { "Cancel" }).clicked() {
                        cancel_revert = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new(if is_de { "Uebernehmen" } else { "Apply" }).color(Color32::WHITE))
                                    .fill(Color32::from_rgb(2, 132, 199)),
                            )
                            .clicked()
                        {
                            apply_close = true;
                        }
                    });
                });
            });
        if cancel_revert || !open {
            if let Some(draft) = app.power_draft.take() {
                app.layout.feed_voltage = draft.feed_voltage;
                app.layout.catenary_feed_voltage = draft.catenary_feed_voltage;
                app.layout.catenary_circuit_mode = draft.catenary_circuit_mode;
                app.layout.loco_current_ma = draft.loco_current_ma;
                app.layout.loco_count = draft.loco_count;
                app.layout.turnout_lantern_current_ma = draft.turnout_lantern_current_ma;
                app.layout.turnout_decoder_current_ma = draft.turnout_decoder_current_ma;
                app.power_sim_enabled = draft.power_sim_enabled;
                app.mark_layout_dirty();
            }
            app.show_power_dialog = false;
        } else if apply_close {
            app.power_draft = None;
            app.show_power_dialog = false;
        }
        if do_add_feeder {
            app.place_feeder_track();
        }
        if do_add_isolator {
            app.place_isolating_track();
        }
        if do_cycle_iso_end {
            app.cycle_selected_isolator_connector();
        }
        if let Some(equip) = bulk_equip {
            app.bulk_equip_turnouts(equip);
        } else if param_changed {
            app.mark_layout_dirty();
        }
        if let Some((x, y)) = focus_pt {
            app.focus_world_point(x, y);
        }
    } else {
        app.power_draft = None;
    }

    if app.show_height_dialog {
        let mut open = app.show_height_dialog;
        let is_de = app.lang == Lang::De;
        let nem_h = min_clearance_for_scale(&app.layout.scale);
        let scale_label = app.layout.scale.clone();
        let layers_list: Vec<(String, String, f64)> = app
            .layout
            .layers
            .as_ref()
            .map(|ls| {
                ls.iter()
                    .map(|l| (l.id.clone(), l.name.clone(), l.height_mm.unwrap_or(0.0)))
                    .collect()
            })
            .unwrap_or_else(|| vec![("layer-1".into(), "Ebene 0".into(), 0.0)]);
        egui::Window::new(t.height_dialog)
            .open(&mut open)
            .resizable(false)
            .default_width(390.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut app.height_tab, 0, if is_de { "Feste Hoehe" } else { "Flat Height" });
                    ui.selectable_value(&mut app.height_tab, 1, "Auto-Ramp");
                    ui.selectable_value(&mut app.height_tab, 2, if is_de { "Rampe / Wendel Gen" } else { "Ramp / Helix Gen" });
                });
                ui.separator();

                match app.height_tab {
                    0 => {
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Hoehe:" } else { "Elevation:" });
                            ui.add(egui::DragValue::new(&mut app.batch_elevation_mm).speed(2.0).suffix(" mm"));
                            if ui.small_button(format!("NEM 102 ({nem_h:.0} mm {scale_label})")).clicked() {
                                app.batch_elevation_mm = nem_h;
                            }
                        });
                        ui.add_space(4.0);
                        if ui
                            .add_enabled(
                                !app.selected.is_empty(),
                                egui::Button::new(if is_de {
                                    "Feste Hoehe auf Auswahl anwenden"
                                } else {
                                    "Apply flat height to selected"
                                }),
                            )
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
                            if !app.selected.is_empty()
                                && ui
                                    .small_button(if is_de { "Nachbarn erkennen" } else { "Infer Neighbors" })
                                    .clicked()
                            {
                                if let Some((_, sh, eh)) =
                                    crate::ramp_tools::calculate_auto_ramp(&app.layout, &app.selected, &app.catalogs)
                                {
                                    app.ramp_start_h = sh;
                                    app.ramp_end_h = eh;
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
                            ui.label(if is_de { "Ende (B):" } else { "End (B):" });
                            ui.add(egui::DragValue::new(&mut app.ramp_end_h).speed(2.0).suffix(" mm"));
                        });
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Max Steigung:" } else { "Max Grade:" });
                            ui.add(egui::DragValue::new(&mut app.ramp_max_grade_limit).range(0.5..=10.0).speed(0.1).suffix(" %"));
                            if !chain.is_empty()
                                && ui
                                    .small_button(if is_de { "Aus Steigung ber." } else { "Calc End H" })
                                    .clicked()
                            {
                                app.ramp_end_h = crate::ramp_tools::calculate_end_height_from_max_gradient(
                                    &app.layout,
                                    &app.catalogs,
                                    &chain,
                                    app.ramp_start_h,
                                    app.ramp_max_grade_limit,
                                    app.ramp_end_h >= app.ramp_start_h,
                                    app.ramp_profile,
                                );
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Profil:" } else { "Profile:" });
                            ui.radio_value(&mut app.ramp_profile, RampProfile::Linear, "Linear");
                            ui.radio_value(&mut app.ramp_profile, RampProfile::Eased, if is_de { "Sanft (S-Kurve)" } else { "Eased (S-Curve)" });
                        });

                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Ziel-Ebene:" } else { "Target Layer:" });
                            let cur_tl_lbl = app
                                .ramp_target_layer_id
                                .as_deref()
                                .and_then(|id| layers_list.iter().find(|(lid, _, _)| lid == id))
                                .map(|(_, n, _)| n.clone())
                                .unwrap_or_else(|| if is_de { "Unveraendert" } else { "Unchanged" }.into());
                            egui::ComboBox::from_id_salt("ramp_target_layer_cb")
                                .width(130.0)
                                .selected_text(cur_tl_lbl)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut app.ramp_target_layer_id, None, if is_de { "Unveraendert" } else { "Unchanged" });
                                    for (lid, lname, lh) in &layers_list {
                                        ui.selectable_value(
                                            &mut app.ramp_target_layer_id,
                                            Some(lid.clone()),
                                            format!("{lname} ({lh:.0} mm)"),
                                        );
                                    }
                                });
                            if ui.small_button("+ Ebene").clicked() {
                                let new_id = app.add_layer_immediate();
                                app.ramp_target_layer_id = Some(new_id);
                            }
                        });

                        if let Some(sum) = preview_auto_ramp_for_chain(
                            &app.layout,
                            &app.catalogs,
                            &chain,
                            app.ramp_start_h,
                            app.ramp_end_h,
                            app.ramp_profile,
                        ) {
                            let exceeds = sum.effective_max_pct > app.ramp_max_grade_limit + 0.05;
                            let grade_col = if exceeds || sum.max_gradient_pct > 4.0 {
                                Color32::from_rgb(239, 68, 68)
                            } else if sum.max_gradient_pct > 3.0 {
                                Color32::from_rgb(245, 158, 11)
                            } else {
                                Color32::from_rgb(34, 197, 94)
                            };
                            let dir_text = format!(
                                "A ({:.0} mm)  →  B ({:.0} mm)  •  {} {}",
                                app.ramp_start_h,
                                app.ramp_end_h,
                                sum.piece_count,
                                if is_de { "Gleise" } else { "pieces" }
                            );
                            ui.label(RichText::new(dir_text).small().color(Color32::from_rgb(56, 189, 248)));
                            ui.label(format!(
                                "{}: {:.1} mm  •  {}: {:.1} mm",
                                if is_de { "Laenge" } else { "Length" },
                                sum.total_length_mm,
                                if is_de { "Hub" } else { "Rise" },
                                sum.total_rise_mm
                            ));
                            ui.label(
                                RichText::new(format!(
                                    "{}: {:.1}% ({} {:.1}%)",
                                    if is_de { "Max Steigung" } else { "Peak grade" },
                                    sum.max_gradient_pct,
                                    if is_de { "Ø" } else { "avg" },
                                    sum.avg_gradient_pct
                                ))
                                .strong()
                                .color(grade_col),
                            );
                            if sum.has_s_curve {
                                ui.label(
                                    RichText::new(if is_de {
                                        format!("⚠ Gegenbogen/S-Kurve erkannt: eff. Reibungssteigung {:.1}% (1.5×)", sum.effective_max_pct)
                                    } else {
                                        format!("⚠ S-Curve detected: effective gradient {:.1}% (1.5× factor)", sum.effective_max_pct)
                                    })
                                    .size(11.0)
                                    .color(Color32::from_rgb(245, 158, 11)),
                                );
                            }
                            if exceeds {
                                ui.label(
                                    RichText::new(if is_de {
                                        format!("⚠ Ueberschreitet Ziel-Steigung ({:.1}% > {:.1}%)", sum.effective_max_pct, app.ramp_max_grade_limit)
                                    } else {
                                        format!("⚠ Exceeds max grade limit ({:.1}% > {:.1}%)", sum.effective_max_pct, app.ramp_max_grade_limit)
                                    })
                                    .size(11.0)
                                    .color(Color32::from_rgb(239, 68, 68)),
                                );
                            }

                            let rows = crate::ramp_tools::build_ramp_breakdown(
                                &app.layout,
                                &app.catalogs,
                                &chain,
                                app.ramp_start_h,
                                app.ramp_end_h,
                                app.ramp_profile,
                            );
                            if !rows.is_empty() {
                                ui.collapsing(
                                    if is_de { "Gleis-Detailtabelle" } else { "Per-Track Breakdown" },
                                    |ui| {
                                        egui::ScrollArea::vertical().max_height(130.0).show(ui, |ui| {
                                            egui::Grid::new("ramp_breakdown_tbl").num_columns(4).striped(true).show(ui, |ui| {
                                                ui.strong("#");
                                                ui.strong(if is_de { "Laenge" } else { "Len" });
                                                ui.strong("Start→End");
                                                ui.strong(if is_de { "Steig." } else { "Grade" });
                                                ui.end_row();
                                                for r in &rows {
                                                    ui.label(&r.catalog_id).on_hover_text(format!("ID: {}", r.piece_id));
                                                    ui.label(format!("{:.0}", r.length_mm));
                                                    ui.label(format!("{:.0}→{:.0}", r.start_h_mm, r.end_h_mm));
                                                    let gtxt = if r.is_s_curve {
                                                        format!("{:.1}% (S)", r.effective_grade_pct)
                                                    } else {
                                                        format!("{:.1}%", r.grade_pct)
                                                    };
                                                    ui.label(gtxt);
                                                    ui.end_row();
                                                }
                                            });
                                        });
                                    },
                                );
                            }
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
                            .add_enabled(
                                !chain.is_empty(),
                                egui::Button::new(if is_de { "Auto-Rampe anwenden" } else { "Apply Auto-Ramp" }),
                            )
                            .clicked()
                        {
                            app.push_undo();
                            if let Some(ref tl) = app.ramp_target_layer_id {
                                for (pid, _) in &chain {
                                    if let Some(pc) = app.layout.pieces.iter_mut().find(|p| &p.id == pid) {
                                        pc.layer_id = Some(tl.clone());
                                    }
                                }
                            }
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
                                    app.show_toast(if is_de {
                                        format!("Rampe auf {} Gleise angewendet ({:.1}%)", sum.piece_count, sum.max_gradient_pct)
                                    } else {
                                        format!("Applied ramp over {} pieces ({:.1}% peak grade)", sum.piece_count, sum.max_gradient_pct)
                                    });
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
                                RichText::new(if is_de {
                                    "1 Startgleis mit offenem Verbinder auswaehlen."
                                } else {
                                    "Select 1 anchor track with an open connector to generate from."
                                })
                                .color(Color32::from_rgb(245, 158, 11)),
                            );
                        } else if let Some(ref aid) = single_sel {
                            let open_conns = crate::ramp_tools::available_open_connectors_for_piece(&app.layout, aid);
                            if open_conns.len() > 1 {
                                ui.horizontal(|ui| {
                                    ui.label(if is_de { "Start-Verbinder:" } else { "Start Connector:" });
                                    for ci in open_conns {
                                        let active = app.gen_conn_idx.unwrap_or(ci) == ci;
                                        if ui.selectable_label(active, format!("#{}", ci + 1)).clicked() {
                                            app.gen_conn_idx = Some(ci);
                                        }
                                    }
                                });
                            }
                        }

                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Form:" } else { "Shape:" });
                            ui.radio_value(&mut app.gen_ramp_shape, RampShape::Straight, if is_de { "Gerade" } else { "Straight" });
                            ui.radio_value(&mut app.gen_ramp_shape, RampShape::Curved, if is_de { "Kurve" } else { "Curved" });
                            ui.radio_value(&mut app.gen_ramp_shape, RampShape::Helix, if is_de { "Wendel" } else { "Helix" });
                        });
                        if app.gen_ramp_shape != RampShape::Straight {
                            ui.horizontal(|ui| {
                                ui.label(if is_de { "Richtung:" } else { "Direction:" });
                                ui.radio_value(&mut app.gen_curve_left, true, if is_de { "Links" } else { "Left" });
                                ui.radio_value(&mut app.gen_curve_left, false, if is_de { "Rechts" } else { "Right" });
                            });
                            if app.gen_ramp_shape == RampShape::Curved {
                                ui.horizontal(|ui| {
                                    ui.label(if is_de { "Bogenwinkel:" } else { "Arc Angle:" });
                                    for deg in [90.0, 180.0, 270.0] {
                                        if ui
                                            .selectable_label((app.gen_arc_angle_deg - deg).abs() < 1.0, format!("{deg:.0}°"))
                                            .clicked()
                                        {
                                            app.gen_arc_angle_deg = deg;
                                        }
                                    }
                                });
                            }
                            if let Some(cat) = app.catalogs.get(app.selected_catalog_idx) {
                                let curves = crate::ramp_tools::available_curve_radii_dedup(cat);
                                if !curves.is_empty() {
                                    let chosen_curve = app
                                        .gen_curve_id
                                        .as_deref()
                                        .and_then(|cid| curves.iter().find(|c| c.id == cid))
                                        .or_else(|| curves.first());
                                    let sel_label = app
                                        .gen_curve_id
                                        .as_deref()
                                        .and_then(|cid| curves.iter().find(|c| c.id == cid))
                                        .map(|c| format!("{} (R{:.0})", c.id, c.radius.unwrap_or(0.0)))
                                        .unwrap_or_else(|| if is_de { "Standard-Radius" } else { "Auto standard curve" }.into());
                                    egui::ComboBox::from_id_salt("gen_curve_combo")
                                        .selected_text(sel_label)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut app.gen_curve_id, None, if is_de { "Standard-Radius" } else { "Auto standard curve" });
                                            for (ri, c) in curves.iter().enumerate() {
                                                let lbl = format!(
                                                    "R{} · #{} ({:.0} mm)",
                                                    ri + 1,
                                                    c.id,
                                                    c.radius.unwrap_or(0.0)
                                                );
                                                ui.selectable_value(&mut app.gen_curve_id, Some(c.id.clone()), lbl);
                                            }
                                        });
                                    if app.gen_ramp_shape == RampShape::Helix {
                                        if let Some(rc) = chosen_curve.and_then(|c| c.radius) {
                                            let circ = 2.0 * std::f64::consts::PI * rc;
                                            ui.small(format!(
                                                "{}: {:.0} mm ({:.2} m / {})",
                                                if is_de { "Wendel-Umfang" } else { "Helix circumference" },
                                                circ,
                                                circ / 1000.0,
                                                if is_de { "Umdrehung" } else { "turn" }
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Richtung:" } else { "Slope:" });
                            if ui.selectable_label(app.gen_slope_up, if is_de { "↗ Aufwaerts" } else { "↗ Up" }).clicked() {
                                app.gen_slope_up = true;
                                app.gen_target_h = app.gen_target_h.abs();
                            }
                            if ui.selectable_label(!app.gen_slope_up, if is_de { "↘ Abwaerts" } else { "↘ Down" }).clicked() {
                                app.gen_slope_up = false;
                                app.gen_target_h = -app.gen_target_h.abs();
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Ziel-Ebene:" } else { "Target Layer:" });
                            let cur_gtl = app
                                .gen_target_layer_id
                                .as_deref()
                                .and_then(|id| layers_list.iter().find(|(lid, _, _)| lid == id))
                                .map(|(_, n, lh)| format!("{n} ({lh:.0} mm)"))
                                .unwrap_or_else(|| if is_de { "Wie Startgleis" } else { "Same as anchor" }.into());
                            egui::ComboBox::from_id_salt("gen_target_layer_cb")
                                .width(140.0)
                                .selected_text(cur_gtl)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut app.gen_target_layer_id, None, if is_de { "Wie Startgleis" } else { "Same as anchor" });
                                    for (lid, lname, lh) in &layers_list {
                                        if ui
                                            .selectable_label(
                                                app.gen_target_layer_id.as_deref() == Some(lid.as_str()),
                                                format!("{lname} ({lh:.0} mm)"),
                                            )
                                            .clicked()
                                        {
                                            app.gen_target_layer_id = Some(lid.clone());
                                            app.gen_target_h = *lh;
                                        }
                                    }
                                });
                            if ui.small_button(format!("+{nem_h:.0} mm")).clicked() {
                                let new_id = app.add_layer_immediate();
                                app.gen_target_layer_id = Some(new_id);
                                app.gen_target_h = if app.gen_slope_up { nem_h } else { -nem_h };
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Zielhoehe:" } else { "Target H:" });
                            ui.add(egui::DragValue::new(&mut app.gen_target_h).speed(2.0).suffix(" mm"));
                            if ui.small_button(format!("±{nem_h:.0}")).clicked() {
                                app.gen_target_h = if app.gen_slope_up { nem_h } else { -nem_h };
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Max Steigung:" } else { "Max Grade:" });
                            ui.add(egui::DragValue::new(&mut app.gen_max_grade).range(0.5..=8.0).speed(0.1).suffix(" %"));
                            if app.gen_ramp_shape == RampShape::Helix {
                                ui.label(if is_de { "Windungen:" } else { "Turns:" });
                                ui.add(egui::DragValue::new(&mut app.gen_helix_turns).range(0.5..=6.0).speed(0.25));
                            }
                        });
                        ui.add_space(4.0);
                        if ui
                            .add_enabled(
                                single_sel.is_some(),
                                egui::Button::new(if is_de {
                                    "Rampe / Wendel erzeugen"
                                } else {
                                    "Generate Ramp / Helix"
                                }),
                            )
                            .clicked()
                        {
                            if let Some(anchor_id) = single_sel {
                                app.push_undo();
                                match crate::ramp_tools::generate_ramp_or_helix_ext(
                                    &mut app.layout,
                                    &app.catalogs,
                                    app.selected_catalog_idx,
                                    &mut app.next_piece_seq,
                                    &anchor_id,
                                    app.gen_conn_idx,
                                    app.gen_target_layer_id.as_deref(),
                                    app.gen_ramp_shape,
                                    app.gen_curve_left,
                                    app.gen_arc_angle_deg,
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
                                        app.show_toast(if is_de {
                                            format!("{} Gleise erzeugt ({:.1}%, {:.1} mm Hub)", sum.piece_count, sum.avg_gradient_pct, sum.total_rise_mm)
                                        } else {
                                            format!("Generated {} pieces ({:.1}% grade, {:.1} mm rise)", sum.piece_count, sum.avg_gradient_pct, sum.total_rise_mm)
                                        });
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
        let is_de = app.lang == Lang::De;
        let title = if is_de {
            "Live-Zusammenarbeit (WebSocket)"
        } else {
            "Live Collaboration (WebSocket)"
        };
        egui::Window::new(title)
            .open(&mut open)
            .resizable(true)
            .default_width(390.0)
            .show(ctx, |ui| {
                // Status & Error banner
                ui.horizontal(|ui| {
                    let dot_color = if app.collab.connected {
                        Color32::from_rgb(16, 185, 129)
                    } else if app.collab.connecting || app.collab.is_server_running() {
                        Color32::from_rgb(245, 158, 11)
                    } else {
                        Color32::from_rgb(148, 163, 184)
                    };
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.5, dot_color);
                    ui.strong(&app.collab.status);
                    if app.collab.is_server_running() {
                        ui.weak(if is_de { "[Host aktiv]" } else { "[Hosting]" });
                    }
                });
                if let Some(err) = &app.collab.error_message {
                    ui.colored_label(Color32::from_rgb(239, 68, 68), err);
                }
                ui.separator();

                // Mode tabs: Host embedded server vs Join external server (Node server.mjs / LAN host)
                ui.horizontal(|ui| {
                    let host_lbl = if is_de { "Server hosten" } else { "Host Server" };
                    let join_lbl = if is_de { "Beitreten" } else { "Join Server" };
                    if ui
                        .selectable_label(app.collab.dialog_tab == crate::collab::CollabDialogTab::Host, host_lbl)
                        .clicked()
                    {
                        app.collab.dialog_tab = crate::collab::CollabDialogTab::Host;
                    }
                    if ui
                        .selectable_label(app.collab.dialog_tab == crate::collab::CollabDialogTab::Join, join_lbl)
                        .clicked()
                    {
                        app.collab.dialog_tab = crate::collab::CollabDialogTab::Join;
                    }
                });
                ui.add_space(4.0);

                egui::Grid::new("collab_fields_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        match app.collab.dialog_tab {
                            crate::collab::CollabDialogTab::Host => {
                                ui.label("Port:");
                                let mut port_u32 = app.collab.host_port as u32;
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut port_u32)
                                            .range(1024..=65535)
                                            .speed(1.0),
                                    )
                                    .changed()
                                {
                                    app.collab.host_port = port_u32 as u16;
                                }
                                ui.end_row();
                            }
                            crate::collab::CollabDialogTab::Join => {
                                ui.label(if is_de { "Server-URL:" } else { "Server URL:" });
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.collab.server_url)
                                        .desired_width(230.0)
                                        .hint_text("ws://localhost:3000/collab"),
                                );
                                ui.end_row();
                            }
                        }

                        ui.label(if is_de { "Raum:" } else { "Room:" });
                        ui.add(
                            egui::TextEdit::singleline(&mut app.collab.room_id)
                                .desired_width(230.0)
                                .hint_text("default"),
                        );
                        ui.end_row();

                        ui.label(if is_de { "Passwort:" } else { "Password:" });
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut app.collab.password)
                                    .password(!app.collab.show_password)
                                    .desired_width(185.0)
                                    .hint_text(if is_de { "optional" } else { "optional" }),
                            );
                            let eye_lbl = if app.collab.show_password { "Hide" } else { "Show" };
                            if ui.small_button(eye_lbl).clicked() {
                                app.collab.show_password = !app.collab.show_password;
                            }
                        });
                        ui.end_row();

                        ui.label(if is_de { "Name:" } else { "User:" });
                        let name_resp = ui.add(
                            egui::TextEdit::singleline(&mut app.collab.user_name)
                                .desired_width(230.0),
                        );
                        if name_resp.lost_focus() {
                            app.collab.send_profile_update();
                            app.save_current_settings();
                        }
                        ui.end_row();
                    });

                // User color picker
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Farbe:" } else { "Color:" });
                    for &hex in crate::collab::COLLAB_COLORS {
                        let c = parse_hex_color(Some(hex)).unwrap_or(Color32::from_rgb(59, 130, 246));
                        let selected = app.collab.user_color == hex;
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
                        ui.painter().circle_filled(rect.center(), 7.5, c);
                        if selected {
                            ui.painter().circle_stroke(
                                rect.center(),
                                8.5,
                                egui::Stroke::new(2.0_f32, Color32::WHITE),
                            );
                        }
                        if resp.clicked() {
                            app.collab.user_color = hex.to_string();
                            app.collab.send_profile_update();
                            app.save_current_settings();
                        }
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    match app.collab.dialog_tab {
                        crate::collab::CollabDialogTab::Host => {
                            if app.collab.is_server_running() {
                                let stop_lbl = if is_de {
                                    "Server stoppen & trennen"
                                } else {
                                    "Stop Server & Disconnect"
                                };
                                if ui.button(stop_lbl).clicked() {
                                    app.collab.stop_host_and_disconnect();
                                }
                            } else {
                                let start_lbl = if is_de {
                                    "Server starten & verbinden"
                                } else {
                                    "Start Server & Connect"
                                };
                                if ui.button(start_lbl).clicked() {
                                    app.collab.start_host_and_connect(Some(&app.layout));
                                }
                            }
                        }
                        crate::collab::CollabDialogTab::Join => {
                            if app.collab.connected || app.collab.connecting {
                                let disc_lbl = if is_de { "Trennen" } else { "Disconnect" };
                                if ui.button(disc_lbl).clicked() {
                                    app.collab.disconnect();
                                }
                            } else {
                                let conn_lbl = if is_de { "Verbinden" } else { "Connect" };
                                if ui.button(conn_lbl).clicked() {
                                    app.collab.connect_with_layout(Some(&app.layout));
                                }
                            }
                        }
                    }
                });

                if let Some((local_ws, lan_ws)) = app.collab.server_urls() {
                    ui.add_space(6.0);
                    let share_url = lan_ws.unwrap_or(local_ws);
                    ui.group(|ui| {
                        ui.label(if is_de {
                            "Einladungs-URL (WebSocket):"
                        } else {
                            "Invite URL (WebSocket):"
                        });
                        ui.horizontal(|ui| {
                            ui.monospace(&share_url);
                            if ui.small_button(if is_de { "Kopieren" } else { "Copy" }).clicked() {
                                ui.ctx().copy_text(share_url.clone());
                                app.show_toast(if is_de {
                                    "Einladungs-URL in Zwischenablage kopiert"
                                } else {
                                    "Invite URL copied to clipboard"
                                });
                            }
                        });
                    });
                }

                if app.collab.connected {
                    ui.separator();
                    let p_title = if is_de {
                        format!("Teilnehmer ({})", 1 + app.collab.users.len())
                    } else {
                        format!("Participants ({})", 1 + app.collab.users.len())
                    };
                    ui.strong(p_title);
                    ui.horizontal_wrapped(|ui| {
                        let me_col = parse_hex_color(Some(&app.collab.user_color))
                            .unwrap_or(Color32::from_rgb(59, 130, 246));
                        ui.colored_label(
                            me_col,
                            format!(
                                "● {} ({})",
                                app.collab.user_name,
                                if is_de { "Du" } else { "You" }
                            ),
                        );
                        for u in &app.collab.users {
                            let uc = parse_hex_color(Some(&u.color))
                                .unwrap_or(Color32::from_rgb(59, 130, 246));
                            ui.colored_label(uc, format!("● {}", u.name));
                        }
                    });

                    ui.add_space(6.0);
                    ui.strong("Chat");
                    egui::ScrollArea::vertical()
                        .max_height(110.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            if app.collab.chat_messages.is_empty() {
                                ui.weak(if is_de {
                                    "Noch keine Nachrichten."
                                } else {
                                    "No messages yet."
                                });
                            }
                            for m in &app.collab.chat_messages {
                                ui.horizontal_wrapped(|ui| {
                                    ui.colored_label(
                                        parse_hex_color(Some(&m.user_color))
                                            .unwrap_or(Color32::from_rgb(59, 130, 246)),
                                        format!("{}:", m.user_name),
                                    );
                                    ui.label(&m.text);
                                });
                            }
                        });

                    ui.horizontal(|ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut app.collab.chat_draft)
                                .desired_width(280.0)
                                .hint_text(if is_de {
                                    "Nachricht schreiben..."
                                } else {
                                    "Type message..."
                                }),
                        );
                        let send_enter =
                            resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let send_btn = ui
                            .button(if is_de { "Senden" } else { "Send" })
                            .clicked();
                        if send_enter || send_btn {
                            let draft = std::mem::take(&mut app.collab.chat_draft);
                            app.collab.send_chat(&draft);
                        }
                    });
                }
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

    if app.show_track_props_piece_id.is_some() {
        draw_track_props_dialog(ctx, app);
    }

    if app.show_replace_track_piece_id.is_some() {
        draw_replace_split_dialog(ctx, app);
    }

    if app.show_layer_dialog {
        draw_layer_edit_dialog(ctx, app);
    }

    if app.confirm_delete_layer_id.is_some() {
        draw_confirm_delete_layer_dialog(ctx, app);
    }

    if app.pending_confirm_action.is_some() {
        draw_confirm_unsaved_dialog(ctx, app);
    }

    if app.show_autosave_recovery {
        draw_autosave_recovery_dialog(ctx, app);
    }
}

