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
            .id(egui::Id::new("railbench_power_dialog"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_width(390.0)
            .frame(crate::theme::railbench_window_frame(app.dark_mode))
            .show(ctx, |ui| {
                crate::theme::show_railbench_dialog_body(ui, app.dark_mode, t.power_dialog, Some(&mut open), 12, |ui| {
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
                            .small_button(if is_de { "↔ Ende" } else { "↔ End" })
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
                    let p = palette(app.dark_mode);
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
                            .color(p.text_warning),
                        );
                    }
                    ui.label(format!(
                        "Feeders: {}  |  Isolators: {}  |  Min Voltage: {:.2} V (Drop {:.1}%)",
                        sim.feeder_count, sim.isolator_count, sim.min_voltage, sim.max_drop_pct
                    ));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("OK {}", sim.optimal_count)).color(p.text_success));
                        ui.label(RichText::new(format!("Warn {}", sim.warning_count)).color(p.text_warning));
                        ui.label(RichText::new(format!("Crit {}", sim.critical_count)).color(p.text_danger));
                        ui.label(RichText::new(format!("Dead {}", sim.disconnected_count)).color(p.text_muted));
                    });
                    if !sim.catenary_recommendation.is_empty() {
                        ui.small(&sim.catenary_recommendation);
                    }
                    if sim.has_short_circuit {
                        ui.add_space(6.0);
                        egui::Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(153, 27, 27, 220))
                            .stroke(egui::Stroke::new(1.5_f32, Color32::from_rgb(252, 165, 165)))
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

    super::height_panel::draw_height_ramp_dialog(ctx, app);

    if app.show_collab_dialog {
        let mut open = app.show_collab_dialog;
        let is_de = app.lang == Lang::De;
        let p = palette(app.dark_mode);
        let title = if is_de {
            "Live-Zusammenarbeit (WebSocket)"
        } else {
            "Live Collaboration (WebSocket)"
        };
        egui::Window::new(title)
            .id(egui::Id::new("railbench_collab_dialog"))
            .title_bar(false)
            .collapsible(false)
            .resizable(true)
            .default_width(390.0)
            .frame(crate::theme::railbench_window_frame(app.dark_mode))
            .show(ctx, |ui| {
                crate::theme::show_railbench_dialog_body(ui, app.dark_mode, title, Some(&mut open), 12, |ui| {
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
                    ui.colored_label(p.text_danger, err);
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
                                "• {} ({})",
                                app.collab.user_name,
                                if is_de { "Du" } else { "You" }
                            ),
                        );
                        for u in &app.collab.users {
                            let uc = parse_hex_color(Some(&u.color))
                                .unwrap_or(Color32::from_rgb(59, 130, 246));
                            ui.colored_label(uc, format!("• {}", u.name));
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

