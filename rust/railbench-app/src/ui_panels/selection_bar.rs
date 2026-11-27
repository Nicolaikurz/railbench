use super::*;

/// Floating pill bar at bottom-center of canvas when 1+ tracks or models are selected (matches src/ui/selection-bar.ts).
pub fn draw_floating_selection_bar(ctx: &egui::Context, app: &mut RailbenchApp) {
    let t = tr(app.lang);
    let p = palette(app.dark_mode);

    // Subtle bottom-right zoom indicator pill (only in 2D mode; 2D zoom percentage does not apply in 3D)
    if !app.is_3d_mode {
        egui::Area::new(egui::Id::new("zoom_hud_pill"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -14.0))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(p.bg_toolbar.r(), p.bg_toolbar.g(), p.bg_toolbar.b(), 215))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::symmetric(10, 4))
                    .stroke(Stroke::new(1.0_f32, p.border_main))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if let Some(msg) = &app.status_message {
                                ui.label(RichText::new(msg).size(11.0).color(Color32::from_rgb(148, 163, 184)));
                                ui.label(RichText::new("•").size(11.0).color(Color32::from_rgb(100, 116, 139)));
                            }
                            ui.label(
                                RichText::new(format!("{:.0}%", app.camera.zoom * 100.0))
                                    .strong()
                                    .size(11.0)
                                    .color(Color32::WHITE),
                            );
                        });
                    });
            });
    }

    if app.selected.is_empty() {
        return;
    }

    let sel_count = app.selected.len();
    let is_de = app.lang == Lang::De;
    let layers_list: Vec<(String, String)> = app
        .layout
        .layers
        .as_ref()
        .map(|ls| ls.iter().map(|l| (l.id.clone(), l.name.clone())).collect())
        .unwrap_or_else(|| {
            vec![(
                "layer-1".into(),
                if is_de { "Ebene 0" } else { "Layer 0" }.into(),
            )]
        });

    let first_sel = app
        .layout
        .pieces
        .iter()
        .find(|pc| app.selected.contains(&pc.id))
        .cloned();

    let Some(sample) = first_sel else {
        // Selected item(s) are 3D buildings/models: show compact model selection bar
        let sel_models: Vec<_> = app
            .layout
            .models
            .as_ref()
            .map(|ms| ms.iter().filter(|m| app.selected.contains(&m.id)).cloned().collect())
            .unwrap_or_default();
        let Some(model) = sel_models.first() else { return };
        let m_lid = model.layer_id.as_deref().unwrap_or("layer-1");
        let all_same_layer = sel_models.iter().all(|m| m.layer_id.as_deref().unwrap_or("layer-1") == m_lid);
        let layer_exists = layers_list.iter().any(|(id, _)| id == m_lid);
        let cur_layer_label = if !layer_exists {
            if app.lang == Lang::De { "(!) Ebene fehlt" } else { "(!) Missing layer" }.to_string()
        } else if !all_same_layer {
            if app.lang == Lang::De { "(Gemischt)" } else { "(Mixed)" }.to_string()
        } else {
            layers_list
                .iter()
                .find(|(id, _)| id == m_lid)
                .map(|(_, n)| n.clone())
                .unwrap_or_else(|| m_lid.to_string())
        };
        let mut move_layer_target: Option<String> = None;

        egui::Area::new(egui::Id::new("floating_selection_bar"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -14.0))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(p.bg_card.r(), p.bg_card.g(), p.bg_card.b(), 242))
                    .corner_radius(CornerRadius::same(20))
                    .inner_margin(Margin::symmetric(12, 6))
                    .stroke(Stroke::new(1.2_f32, p.border_main))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let lbl = if sel_count == 1 {
                                model.name.clone()
                            } else {
                                format!("{sel_count}×")
                            };
                            ui.label(RichText::new(lbl).strong().size(12.0).color(p.primary));
                            ui.separator();

                            egui::ComboBox::from_id_salt("sel_model_layer_combo")
                                .width(92.0)
                                .selected_text(&cur_layer_label)
                                .show_ui(ui, |ui| {
                                    for (lid, lname) in &layers_list {
                                        if ui.selectable_label(all_same_layer && layer_exists && lid == m_lid, lname).clicked() {
                                            move_layer_target = Some(lid.clone());
                                        }
                                    }
                                });
                            ui.separator();

                            let mut m_elev = model.elevation.unwrap_or(0.0);
                            let mut m_scale = model.scale.unwrap_or(1.0);
                            let mut m_rot = model.transform.rotation;
                            ui.label(RichText::new("Z:").size(11.0).color(p.text_muted));
                            if ui.add(egui::DragValue::new(&mut m_elev).speed(1.0).suffix(" mm")).changed() {
                                if let Some(ms) = &mut app.layout.models {
                                    for m in ms {
                                        if app.selected.contains(&m.id) {
                                            m.elevation = Some(m_elev);
                                        }
                                    }
                                }
                                app.mark_layout_dirty();
                            }
                            ui.label(RichText::new("Scale:").size(11.0).color(p.text_muted));
                            if ui.add(egui::DragValue::new(&mut m_scale).range(0.1..=20.0).speed(0.05).suffix("x")).changed() {
                                if let Some(ms) = &mut app.layout.models {
                                    for m in ms {
                                        if app.selected.contains(&m.id) {
                                            m.scale = Some(m_scale);
                                        }
                                    }
                                }
                                app.mark_layout_dirty();
                            }
                            ui.label(RichText::new("Rot:").size(11.0).color(p.text_muted));
                            if ui.add(egui::DragValue::new(&mut m_rot).range(-180.0..=360.0).speed(1.0).suffix(" deg")).changed() {
                                if let Some(ms) = &mut app.layout.models {
                                    for m in ms {
                                        if app.selected.contains(&m.id) {
                                            m.transform.rotation = railbench_core::geometry::norm_angle_deg(m_rot);
                                        }
                                    }
                                }
                                app.mark_layout_dirty();
                            }
                            ui.separator();

                            let sz = egui::vec2(26.0, 24.0);
                            if icon_button_sized(ui, Icon::RotateCcw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_ccw).clicked() {
                                app.rotate_selected(-app.rotation_step);
                            }
                            if icon_button_sized(ui, Icon::RotateCw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_cw).clicked() {
                                app.rotate_selected(app.rotation_step);
                            }
                            if icon_button_sized(
                                ui,
                                Icon::Trash,
                                sz,
                                true,
                                p.bg_input,
                                Color32::from_rgb(220, 38, 38),
                                Color32::from_rgb(239, 68, 68),
                                Color32::WHITE,
                                t.delete_sel,
                            )
                            .clicked()
                            {
                                app.delete_selected();
                            }
                        });
                    });
            });
        if let Some(tid) = move_layer_target {
            app.move_selected_to_layer(&tid);
        }
        return;
    };

    let selected_pieces: Vec<_> = app
        .layout
        .pieces
        .iter()
        .filter(|pc| app.selected.contains(&pc.id))
        .cloned()
        .collect();

    let first_lid = sample.layer_id.as_deref().unwrap_or("layer-1");
    let all_same_layer = selected_pieces
        .iter()
        .all(|pc| pc.layer_id.as_deref().unwrap_or("layer-1") == first_lid);
    let layer_exists = layers_list.iter().any(|(id, _)| id == first_lid);
    let cur_layer_label = if !layer_exists {
        if app.lang == Lang::De { "(!) Ebene fehlt" } else { "(!) Missing layer" }.to_string()
    } else if !all_same_layer {
        if app.lang == Lang::De { "(Gemischt)" } else { "(Mixed)" }.to_string()
    } else {
        layers_list
            .iter()
            .find(|(id, _)| id == first_lid)
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| first_lid.to_string())
    };
    let mut move_layer_target: Option<String> = None;

    let any_turnout = selected_pieces.iter().any(|pc| {
        crate::catalog::find_entry(&app.catalogs, &pc.catalog_id)
            .map_or(false, |e| crate::is_turnout_kind(&e.kind))
    });
    let has_feeder_all = selected_pieces.iter().all(|pc| pc.power_feed == Some(true));
    let has_feeder_any = selected_pieces.iter().any(|pc| pc.power_feed == Some(true));
    let has_iso_all = selected_pieces.iter().all(|pc| pc.is_isolator == Some(true));
    let has_iso_any = selected_pieces.iter().any(|pc| pc.is_isolator == Some(true));
    let has_cat_all = selected_pieces.iter().all(|pc| pc.has_catenary == Some(true));
    let has_cat_any = selected_pieces.iter().any(|pc| pc.has_catenary == Some(true));
    let has_cat_feed_all = selected_pieces.iter().all(|pc| pc.catenary_power_feed == Some(true));
    let has_cat_feed_any = selected_pieces.iter().any(|pc| pc.catenary_power_feed == Some(true));
    let has_cat_iso_all = selected_pieces.iter().all(|pc| pc.catenary_isolator == Some(true));
    let has_cat_iso_any = selected_pieces.iter().any(|pc| pc.catenary_isolator == Some(true));
    let has_sig_all = selected_pieces.iter().all(|pc| pc.has_signal == Some(true));
    let has_sig_any = selected_pieces.iter().any(|pc| pc.has_signal == Some(true));
    let any_sig = has_sig_any;
    let sig_side = selected_pieces
        .iter()
        .find(|pc| pc.has_signal == Some(true))
        .and_then(|pc| pc.signal_side.as_deref())
        .unwrap_or("right");
    let sig_dir = selected_pieces
        .iter()
        .find(|pc| pc.has_signal == Some(true))
        .and_then(|pc| pc.signal_direction.as_deref())
        .unwrap_or("forward");
    let has_lantern_all = selected_pieces.iter().filter(|pc| {
        crate::catalog::find_entry(&app.catalogs, &pc.catalog_id).map_or(false, |e| crate::is_turnout_kind(&e.kind))
    }).all(|pc| pc.has_lantern == Some(true));
    let has_lantern_any = selected_pieces.iter().any(|pc| pc.has_lantern == Some(true));
    let has_decoder_all = selected_pieces.iter().filter(|pc| {
        crate::catalog::find_entry(&app.catalogs, &pc.catalog_id).map_or(false, |e| crate::is_turnout_kind(&e.kind))
    }).all(|pc| pc.has_decoder == Some(true));
    let has_decoder_any = selected_pieces.iter().any(|pc| pc.has_decoder == Some(true));
    let has_bridge_all = selected_pieces.iter().all(|pc| pc.has_bridge == Some(true));
    let has_bridge_any = selected_pieces.iter().any(|pc| pc.has_bridge == Some(true));
    let has_tunnel_all = selected_pieces.iter().all(|pc| pc.has_tunnel == Some(true));
    let has_tunnel_any = selected_pieces.iter().any(|pc| pc.has_tunnel == Some(true));
    let layer_h = app
        .layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == first_lid))
        .and_then(|l| l.height_mm)
        .unwrap_or(0.0);
    let mut z_mm = sample.elevation.map(|e| e.start).unwrap_or(0.0);
    let mut track_rot_deg = railbench_core::geometry::norm_angle_deg(sample.transform.rotation);

    egui::Area::new(egui::Id::new("floating_selection_bar"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -14.0))
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(p.bg_card.r(), p.bg_card.g(), p.bg_card.b(), 242))
                .corner_radius(CornerRadius::same(20))
                .inner_margin(Margin::symmetric(12, 6))
                .stroke(Stroke::new(1.2_f32, p.border_main))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 6],
                    blur: 18,
                    spread: 0,
                    color: Color32::from_black_alpha(80),
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Selection count + ID + piece type badge
                        let is_de = app.lang == Lang::De;
                        let badge_txt = if selected_pieces.len() == 1 {
                            let entry_name = crate::catalog::find_entry(&app.catalogs, &sample.catalog_id)
                                .map(|e| {
                                    let n = if !is_de { e.name_en.as_deref().unwrap_or(&e.name) } else { e.name.as_str() };
                                    if n.chars().count() > 16 {
                                        let short: String = n.chars().take(15).collect();
                                        format!("{short}…")
                                    } else {
                                        n.to_string()
                                    }
                                })
                                .unwrap_or_default();
                            if entry_name.is_empty() {
                                sample.catalog_id.clone()
                            } else {
                                format!("{} · {}", sample.catalog_id, entry_name)
                            }
                        } else {
                            format!("{sel_count}×")
                        };
                        ui.label(RichText::new(badge_txt).strong().size(12.0).color(p.primary));
                        if sample.group_id.is_some() {
                            ui.label(RichText::new("[G]").strong().size(10.5).color(p.primary))
                                .on_hover_text(if is_de { "Gruppiert (G zum Aufheben)" } else { "Grouped (G to ungroup)" });
                        }
                        ui.separator();

                        // Layer dropdown (Move selected to layer)
                        egui::ComboBox::from_id_salt("sel_bar_layer_combo")
                            .width(88.0)
                            .selected_text(&cur_layer_label)
                            .show_ui(ui, |ui| {
                                for (lid, lname) in &layers_list {
                                    if ui.selectable_label(all_same_layer && layer_exists && lid == first_lid, lname).clicked() {
                                        move_layer_target = Some(lid.clone());
                                    }
                                }
                            });
                        ui.separator();

                        // Flextrack / Flex-street Length, Radius & Bend Angle controls
                        if sel_count == 1 && crate::catalog::is_flex_id(&app.catalogs, &sample.catalog_id) {
                            let default_len = crate::catalog::find_entry(&app.catalogs, &sample.catalog_id)
                                .and_then(|e| e.length)
                                .unwrap_or(900.0);
                            let mut flex_len = sample.custom_length.unwrap_or(default_len);
                            let mut flex_rad = sample.custom_radius.unwrap_or(0.0);
                            let mut flex_ang = sample.custom_angle.unwrap_or(0.0);
                            let mut flex_changed = false;
                            let mut radius_changed = false;

                            ui.label(RichText::new("L:").size(11.5).color(p.text_muted));
                            let l_resp = ui.add(
                                egui::DragValue::new(&mut flex_len)
                                    .range(25.0..=default_len.max(1200.0))
                                    .speed(2.0)
                                    .suffix(" mm"),
                            );
                            if l_resp.drag_started() {
                                app.push_undo();
                            }
                            if l_resp.changed() {
                                if !l_resp.dragged() {
                                    app.push_undo();
                                }
                                flex_changed = true;
                            }

                            ui.label(RichText::new("R:").size(11.5).color(p.text_muted));
                            let r_resp = ui.add(
                                egui::DragValue::new(&mut flex_rad)
                                    .range(0.0..=9999.0)
                                    .speed(5.0)
                                    .suffix(" mm"),
                            );
                            if r_resp.drag_started() {
                                app.push_undo();
                            }
                            if r_resp.changed() {
                                if !r_resp.dragged() {
                                    app.push_undo();
                                }
                                radius_changed = true;
                            }

                            ui.label(RichText::new("W:").size(11.5).color(p.text_muted));
                            let w_resp = ui.add(
                                egui::DragValue::new(&mut flex_ang)
                                    .range(-180.0..=180.0)
                                    .speed(1.0)
                                    .suffix("°"),
                            );
                            if w_resp.drag_started() {
                                app.push_undo();
                            }
                            if w_resp.changed() {
                                if !w_resp.dragged() {
                                    app.push_undo();
                                }
                                flex_changed = true;
                            }

                            let ri = crate::catalog::get_flex_track_ramp_info(&sample, &app.catalogs);
                            if ri.is_ramp {
                                ui.label(
                                    RichText::new(format!("{:.1}%", ri.gradient_pct))
                                        .size(11.0)
                                        .color(if ri.gradient_pct.abs() > 3.5 {
                                            p.text_danger
                                        } else {
                                            p.text_muted
                                        }),
                                )
                                .on_hover_text(format!(
                                    "L={:.0} mm · {:.0}->{:.0} mm (ΔZ={:.1} mm)",
                                    ri.length_mm, ri.start_elev_mm, ri.end_elev_mm, ri.rise_mm
                                ));
                            }
                            if ui.small_button("Reset").on_hover_text(if is_de { "Flexgleis gerade zuruecksetzen (Anker bleibt)" } else { "Reset flex to straight (keeps connected anchor)" }).clicked() {
                                app.push_undo();
                                if let Some(pc) = app.layout.pieces.iter_mut().find(|x| x.id == sample.id) {
                                    crate::catalog::reset_flex_track_to_original(pc, &app.catalogs);
                                }
                                railbench_core::snapping::update_layout_connections(&mut app.layout, 4.0, 35.0, 1.0);
                                crate::catalog::disconnect_cross_type_connections(&mut app.layout, &app.catalogs);
                                app.mark_layout_dirty();
                            } else if radius_changed {
                                if let Some(pc) = app.layout.pieces.iter_mut().find(|x| x.id == sample.id) {
                                    crate::catalog::bend_flex_track(pc, &app.catalogs, flex_rad, None);
                                }
                                railbench_core::snapping::update_layout_connections(&mut app.layout, 4.0, 35.0, 1.0);
                                crate::catalog::disconnect_cross_type_connections(&mut app.layout, &app.catalogs);
                                app.mark_layout_dirty();
                            } else if flex_changed {
                                if let Some(pc) = app.layout.pieces.iter_mut().find(|x| x.id == sample.id) {
                                    crate::catalog::apply_flex_geometry(pc, &app.catalogs, flex_len, flex_ang);
                                }
                                railbench_core::snapping::update_layout_connections(&mut app.layout, 4.0, 35.0, 1.0);
                                crate::catalog::disconnect_cross_type_connections(&mut app.layout, &app.catalogs);
                                app.mark_layout_dirty();
                            }
                            ui.separator();
                        }

                        // Clickable Z label (opens Height & Ramp dialog) + quick Z elevation input + ramp range readout
                        if ui
                            .small_button("Z:")
                            .on_hover_text(if is_de { "Hoehen & Rampen-Dialog oeffnen (M)" } else { "Open Height & Ramp dialog (M)" })
                            .clicked()
                        {
                            app.show_height_dialog = true;
                        }
                        let z_resp = ui.add(egui::DragValue::new(&mut z_mm).speed(1.0).suffix(" mm"));
                        if z_resp.drag_started() {
                            app.push_undo();
                        }
                        if z_resp.changed() {
                            if !z_resp.dragged() {
                                app.push_undo();
                            }
                            for pc in &mut app.layout.pieces {
                                if app.selected.contains(&pc.id) {
                                    pc.elevation = Some(Elevation { start: z_mm, end: z_mm });
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
                        if let Some(el) = sample.elevation {
                            if (el.end - el.start).abs() > 0.2 {
                                ui.label(
                                    RichText::new(format!("({:.0}->{:.0})", layer_h + el.start, layer_h + el.end))
                                        .size(10.5)
                                        .color(p.text_muted),
                                );
                            }
                        }

                        // Numeric track Rotation input
                        ui.label(RichText::new("Rot:").size(11.5).color(p.text_muted));
                        let prev_rot = track_rot_deg;
                        let rot_resp = ui.add(
                            egui::DragValue::new(&mut track_rot_deg)
                                .range(-180.0..=360.0)
                                .speed(1.0)
                                .suffix("°"),
                        );
                        if rot_resp.drag_started() {
                            app.sel_bar_drag_undo_pushed = false;
                        }
                        if rot_resp.changed() {
                            let delta = track_rot_deg - prev_rot;
                            if delta.abs() > 1e-4 {
                                if !rot_resp.dragged() || !app.sel_bar_drag_undo_pushed {
                                    app.rotate_selected(delta);
                                    app.sel_bar_drag_undo_pushed = true;
                                } else {
                                    // Direct incremental rotation without spamming undo frames during continuous drag
                                    for pc in &mut app.layout.pieces {
                                        if app.selected.contains(&pc.id) {
                                            let px = pc.transform.x;
                                            let py = pc.transform.y;
                                            pc.transform.rotation = railbench_core::geometry::norm_angle_deg(pc.transform.rotation + delta);
                                            for c in &mut pc.connectors {
                                                let rel = railbench_core::geometry::rotate(
                                                    railbench_core::types::Vec2 { x: c.x - px, y: c.y - py },
                                                    delta,
                                                );
                                                c.x = px + rel.x;
                                                c.y = py + rel.y;
                                                c.angle = railbench_core::geometry::norm_angle_deg(c.angle + delta);
                                                c.connected_to = None;
                                            }
                                        }
                                    }
                                    railbench_core::snapping::update_layout_connections(&mut app.layout, 4.0, 35.0, 1.0);
                                    crate::catalog::disconnect_cross_type_connections(&mut app.layout, &app.catalogs);
                                    app.mark_layout_dirty();
                                }
                            }
                        }
                        ui.separator();

                        let toggle_pill_tri = |ui: &mut egui::Ui, label: &str, all_on: bool, any_on: bool, active_col: Color32, tip: &str| -> bool {
                            let mixed = any_on && !all_on;
                            let fill = if all_on {
                                active_col
                            } else if mixed {
                                Color32::from_rgba_unmultiplied(245, 158, 11, 36)
                            } else {
                                p.bg_input
                            };
                            let txt_col = if all_on {
                                Color32::WHITE
                            } else if mixed {
                                p.text_warning
                            } else {
                                p.text_sub
                            };
                            let border_col = if all_on {
                                active_col
                            } else if mixed {
                                Color32::from_rgb(245, 158, 11)
                            } else {
                                p.border_main
                            };
                            let display_lbl = if mixed {
                                format!("~ {label}")
                            } else {
                                label.to_string()
                            };
                            ui.add(
                                egui::Button::new(RichText::new(display_lbl).strong().size(11.0).color(txt_col))
                                    .fill(fill)
                                    .stroke(Stroke::new(1.0_f32, border_col))
                                    .corner_radius(CornerRadius::same(9)),
                            )
                            .on_hover_text(tip)
                            .clicked()
                        };

                        let sz = egui::vec2(26.0, 24.0);
                        let feeder_txt = if is_de { "⚡ Einspeiser" } else { "⚡ Feeder" };
                        if toggle_pill_tri(ui, feeder_txt, has_feeder_all, has_feeder_any, Color32::from_rgb(217, 119, 6), t.feeder) {
                            app.toggle_selected_feeder();
                        }
                        let iso_lbl = if sel_count == 1 && has_iso_all {
                            let idx = sample.isolated_connector_index.unwrap_or(0) + 1;
                            let total = sample.connectors.len().max(2);
                            if is_de {
                                format!("✂ Trenn {idx}/{total}")
                            } else {
                                format!("✂ Iso {idx}/{total}")
                            }
                        } else if is_de {
                            "✂ Trennstelle".into()
                        } else {
                            "✂ Isolator".into()
                        };
                        if toggle_pill_tri(ui, &iso_lbl, has_iso_all, has_iso_any, Color32::from_rgb(220, 38, 38), t.isolator) {
                            app.toggle_selected_isolator();
                        }
                        if has_iso_any && toggle_pill_tri(ui, "↔", false, false, Color32::from_rgb(185, 28, 28), if is_de { "Isoliertes Gleisende wechseln" } else { "Cycle isolated track end" }) {
                            app.cycle_selected_isolator_connector();
                        }

                        // Compact Catenary popover trigger (opens above bar; reveals Feeder/Isolator when Catenary active)
                        let cat_group_active = has_cat_any || has_cat_feed_any || has_cat_iso_any;
                        let cat_popup_id = egui::Id::new("sel_bar_catenary_popup");
                        let cat_open = ui.memory(|m| m.is_popup_open(cat_popup_id));
                        let cat_fill = if has_cat_all {
                            p.primary_hover
                        } else if cat_group_active {
                            Color32::from_rgba_unmultiplied(2, 132, 199, 45)
                        } else {
                            p.bg_input
                        };
                        let cat_txt_col = if has_cat_all { Color32::WHITE } else { p.text_sub };
                        let cat_border = if cat_group_active || cat_open {
                            p.primary
                        } else {
                            p.border_main
                        };
                        let cat_resp = ui
                            .add(
                                egui::Button::new(RichText::new(format!("{}   ", t.catenary)).strong().size(11.0).color(cat_txt_col))
                                    .fill(cat_fill)
                                    .stroke(Stroke::new(1.0_f32, cat_border))
                                    .corner_radius(CornerRadius::same(9)),
                            )
                            .on_hover_text(if is_de { "Oberleitung & OL-Speisung/Trenner" } else { "Catenary & Overhead Power/Isolator" });
                        if ui.is_rect_visible(cat_resp.rect) {
                            crate::icons::paint_chevron(
                                ui.painter(),
                                egui::pos2(cat_resp.rect.max.x - 9.0, cat_resp.rect.center().y),
                                crate::icons::ChevronDir::Up,
                                2.7,
                                cat_txt_col,
                            );
                        }
                        if cat_resp.clicked() {
                            ui.memory_mut(|m| m.toggle_popup(cat_popup_id));
                        }
                        egui::popup::popup_above_or_below_widget(
                            ui,
                            cat_popup_id,
                            &cat_resp,
                            egui::AboveOrBelow::Above,
                            egui::PopupCloseBehavior::CloseOnClickOutside,
                            |ui| {
                                ui.set_min_width(185.0);
                                if toggle_pill_tri(ui, t.catenary, has_cat_all, has_cat_any, p.primary_hover, "Catenary (O)") {
                                    app.toggle_selected_catenary();
                                }
                                if has_cat_any {
                                    ui.separator();
                                    let ol_feed_lbl = if is_de { "OL-Speisung (OL+)" } else { "Catenary Feeder (OL+)" };
                                    if toggle_pill_tri(ui, ol_feed_lbl, has_cat_feed_all, has_cat_feed_any, Color32::from_rgb(2, 132, 199), "Catenary Feeder (Shift+F)") {
                                        app.toggle_selected_catenary_feeder();
                                    }
                                    let ol_iso_lbl = if sel_count == 1 && has_cat_iso_all {
                                        let idx = sample.catenary_isolated_connector_index.unwrap_or(0) + 1;
                                        let total = sample.connectors.len().max(2);
                                        if is_de { format!("OL-Trenner {idx}/{total}") } else { format!("Catenary Isolator {idx}/{total}") }
                                    } else if is_de {
                                        "OL-Trenner (OL/)".into()
                                    } else {
                                        "Catenary Isolator (OL/)".into()
                                    };
                                    if toggle_pill_tri(ui, &ol_iso_lbl, has_cat_iso_all, has_cat_iso_any, Color32::from_rgb(220, 38, 38), "Catenary Isolator (Shift+I)") {
                                        app.toggle_selected_catenary_isolator();
                                    }
                                }
                            },
                        );

                        // Compact Accessories popover trigger (Signal + Side/Dir + Turnout Lantern/Decoder)
                        let acc_active_cnt = usize::from(any_sig)
                            + usize::from(any_turnout && has_lantern_any)
                            + usize::from(any_turnout && has_decoder_any);
                        let acc_popup_id = egui::Id::new("sel_bar_accessories_popup");
                        let acc_open = ui.memory(|m| m.is_popup_open(acc_popup_id));
                        let acc_base = if is_de { "Zubehoer" } else { "Accessories" };
                        let acc_lbl = if acc_active_cnt > 0 {
                            format!("{acc_base} ({acc_active_cnt})   ")
                        } else {
                            format!("{acc_base}   ")
                        };
                        let acc_fill = if acc_active_cnt > 0 { p.primary_hover } else { p.bg_input };
                        let acc_txt_col = if acc_active_cnt > 0 { Color32::WHITE } else { p.text_sub };
                        let acc_border = if acc_active_cnt > 0 || acc_open { p.primary } else { p.border_main };
                        let acc_resp = ui
                            .add(
                                egui::Button::new(RichText::new(acc_lbl).strong().size(11.0).color(acc_txt_col))
                                    .fill(acc_fill)
                                    .stroke(Stroke::new(1.0_f32, acc_border))
                                    .corner_radius(CornerRadius::same(9)),
                            )
                            .on_hover_text(if is_de { "Signale, Weichenlaterne & Decoder" } else { "Signals, Turnout Lantern & Decoder" });
                        if ui.is_rect_visible(acc_resp.rect) {
                            crate::icons::paint_chevron(
                                ui.painter(),
                                egui::pos2(acc_resp.rect.max.x - 9.0, acc_resp.rect.center().y),
                                crate::icons::ChevronDir::Up,
                                2.7,
                                acc_txt_col,
                            );
                        }
                        if acc_resp.clicked() {
                            ui.memory_mut(|m| m.toggle_popup(acc_popup_id));
                        }
                        egui::popup::popup_above_or_below_widget(
                            ui,
                            acc_popup_id,
                            &acc_resp,
                            egui::AboveOrBelow::Above,
                            egui::PopupCloseBehavior::CloseOnClickOutside,
                            |ui| {
                                ui.set_min_width(190.0);
                                let sig_lbl = if is_de { "🚦 Signal (Shift+S)" } else { "🚦 Signal (Shift+S)" };
                                if toggle_pill_tri(ui, sig_lbl, has_sig_all, has_sig_any, Color32::from_rgb(22, 163, 74), "Signal (Shift+S)") {
                                    app.toggle_selected_signal();
                                }
                                if any_sig {
                                    ui.horizontal(|ui| {
                                        let side_lbl = match sig_side {
                                            "left" => "L",
                                            "both" => "L+R",
                                            _ => "R",
                                        };
                                        if toggle_pill_tri(
                                            ui,
                                            side_lbl,
                                            true,
                                            true,
                                            Color32::from_rgb(15, 118, 110),
                                            if is_de { "Signalseite wechseln" } else { "Cycle Signal Side" },
                                        ) {
                                            app.cycle_selected_signal_side();
                                        }
                                        let dir_lbl = match sig_dir {
                                            "backward" => "<-",
                                            "both" => "<->",
                                            _ => "->",
                                        };
                                        if toggle_pill_tri(
                                            ui,
                                            dir_lbl,
                                            true,
                                            true,
                                            Color32::from_rgb(15, 118, 110),
                                            if is_de { "Signalrichtung wechseln" } else { "Cycle Signal Direction" },
                                        ) {
                                            app.cycle_selected_signal_direction();
                                        }
                                    });
                                }
                                if any_turnout {
                                    ui.separator();
                                    let lantern_lbl = if is_de { "Weichenlaterne (W)" } else { "Turnout Lantern (W)" };
                                    if toggle_pill_tri(
                                        ui,
                                        lantern_lbl,
                                        has_lantern_all,
                                        has_lantern_any,
                                        Color32::from_rgb(202, 138, 4),
                                        lantern_lbl,
                                    ) {
                                        app.toggle_selected_lantern();
                                    }
                                    let dec_lbl = if is_de { "Weichendecoder (DEC)" } else { "Turnout Decoder (DEC)" };
                                    let dec_tip = if is_de {
                                        "Weichendecoder (Shift+W / Shift+D)"
                                    } else {
                                        "Turnout Decoder (Shift+W / Shift+D)"
                                    };
                                    if toggle_pill_tri(ui, dec_lbl, has_decoder_all, has_decoder_any, Color32::from_rgb(2, 132, 199), dec_tip) {
                                        app.toggle_selected_decoder();
                                    }
                                }
                            },
                        );

                        if toggle_pill_tri(
                            ui,
                            t.bridge,
                            has_bridge_all,
                            has_bridge_any,
                            p.primary_hover,
                            if is_de { "Brücke (B)" } else { "Bridge (B)" },
                        ) {
                            app.toggle_selected_bridge();
                        }
                        if toggle_pill_tri(
                            ui,
                            t.tunnel,
                            has_tunnel_all,
                            has_tunnel_any,
                            p.primary_hover,
                            if is_de { "Tunnel (T)" } else { "Tunnel (T)" },
                        ) {
                            app.toggle_selected_tunnel();
                        }

                        ui.separator();
                        if toggle_pill_tri(ui, "↔", false, false, p.primary_hover, if is_de { "Spiegeln / Wenden (Alt+F)" } else { "Mirror / Flip (Alt+F)" }) {
                            app.flip_selected();
                        }
                        if icon_button_sized(ui, Icon::RotateCcw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_ccw).clicked() {
                            app.rotate_selected(-app.rotation_step);
                        }
                        if icon_button_sized(ui, Icon::RotateCw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_cw).clicked() {
                            app.rotate_selected(app.rotation_step);
                        }
                        if icon_button_sized(
                            ui,
                            Icon::Trash,
                            sz,
                            true,
                            p.bg_input,
                            Color32::from_rgb(220, 38, 38),
                            Color32::from_rgb(239, 68, 68),
                            Color32::WHITE,
                            t.delete_sel,
                        )
                        .clicked()
                        {
                            app.delete_selected();
                        }
                    });
                });
        });

    if let Some(tid) = move_layer_target {
        app.move_selected_to_layer(&tid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Pos2, RawInput, Rect};

    #[test]
    fn selection_bar_renders_in_2d_and_3d_views() {
        let mut app = RailbenchApp::default();
        app.layout = crate::make_empty_layout();
        let vp = Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0));

        // Populate 1 track piece and 1 3D model
        let entry = crate::catalog::find_entry(&app.catalogs, "24188")
            .cloned()
            .expect("24188 track catalog entry");
        let piece = crate::catalog::create_piece(
            "t-sel-1".into(),
            &entry,
            railbench_core::types::Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            Some("layer-1".into()),
        );
        app.layout.pieces.push(piece);
        app.layout.models = Some(vec![railbench_core::types::Model3D {
            id: "m-sel-1".into(),
            name: "Test Station".into(),
            category: "Buildings".into(),
            transform: railbench_core::types::Transform { x: 120.0, y: 80.0, rotation: 0.0 },
            elevation: None,
            scale: Some(1.0),
            layer_id: Some("layer-1".into()),
            width: 120.0,
            depth: 60.0,
            height: 80.0,
            model_type: "builtin".into(),
            builtin_id: Some("station-small".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        }]);

        // 1. Track selected in 3D mode: unified floating_selection_bar shows, 2D zoom_hud_pill does NOT
        app.selected.clear();
        app.selected.insert("t-sel-1".into());
        app.is_3d_mode = true;
        let ctx_3d_track = egui::Context::default();
        let mut in_3d = RawInput::default();
        in_3d.screen_rect = Some(vp);
        let _ = ctx_3d_track.run(in_3d, |c| draw_floating_selection_bar(c, &mut app));
        assert!(
            ctx_3d_track
                .memory(|m| m.area_rect(egui::Id::new("floating_selection_bar")))
                .is_some(),
            "floating_selection_bar should render for selected track in 3D mode"
        );
        assert!(
            ctx_3d_track
                .memory(|m| m.area_rect(egui::Id::new("zoom_hud_pill")))
                .is_none(),
            "2D zoom_hud_pill should not render in 3D mode"
        );

        // 2. Track selected in 2D mode: both floating_selection_bar and zoom_hud_pill render
        app.is_3d_mode = false;
        let ctx_2d_track = egui::Context::default();
        let mut in_2d = RawInput::default();
        in_2d.screen_rect = Some(vp);
        let _ = ctx_2d_track.run(in_2d, |c| draw_floating_selection_bar(c, &mut app));
        assert!(
            ctx_2d_track
                .memory(|m| m.area_rect(egui::Id::new("floating_selection_bar")))
                .is_some(),
            "floating_selection_bar should render for selected track in 2D mode"
        );
        assert!(
            ctx_2d_track
                .memory(|m| m.area_rect(egui::Id::new("zoom_hud_pill")))
                .is_some(),
            "zoom_hud_pill should render in 2D mode"
        );

        // 3. 3D model selected in 3D mode: unified model floating_selection_bar renders cleanly
        app.selected.clear();
        app.selected.insert("m-sel-1".into());
        app.is_3d_mode = true;
        let ctx_3d_model = egui::Context::default();
        let mut in_3d_m = RawInput::default();
        in_3d_m.screen_rect = Some(vp);
        let _ = ctx_3d_model.run(in_3d_m, |c| draw_floating_selection_bar(c, &mut app));
        assert!(
            ctx_3d_model
                .memory(|m| m.area_rect(egui::Id::new("floating_selection_bar")))
                .is_some(),
            "floating_selection_bar should render for selected 3D model in 3D mode"
        );
        assert!(
            ctx_3d_model
                .memory(|m| m.area_rect(egui::Id::new("zoom_hud_pill")))
                .is_none(),
            "2D zoom_hud_pill should not render when a 3D model is selected in 3D mode"
        );
    }
}

