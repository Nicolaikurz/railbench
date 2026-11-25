use super::*;

pub fn draw_right_sidebar(ctx: &egui::Context, app: &mut RailbenchApp, viewport_rect: egui::Rect) {
    if !app.show_right_sidebar {
        return;
    }
    let t = tr(app.lang);
    let p = palette(app.dark_mode);
    let bg_alpha = if app.dark_mode { 194 } else { 212 };
    let card_alpha = if app.dark_mode { 180 } else { 198 };
    let card_fill = Color32::from_rgba_unmultiplied(p.bg_card.r(), p.bg_card.g(), p.bg_card.b(), card_alpha);
    let win_fill = Color32::from_rgba_unmultiplied(p.bg_sidebar.r(), p.bg_sidebar.g(), p.bg_sidebar.b(), bg_alpha);
    let win_border = Color32::from_rgba_unmultiplied(p.border_main.r(), p.border_main.g(), p.border_main.b(), 215);

    let mut open = app.show_right_sidebar;
    let mut visibility_or_layers_changed = false;
    let max_w = (viewport_rect.width() * 0.45).max(260.0);
    let max_h = (viewport_rect.height() - 56.0).max(160.0);

    egui::Window::new(RichText::new(t.layers_title.to_uppercase()).strong().size(11.5).color(p.text_main))
        .id(egui::Id::new("floating_layers_panel"))
        .open(&mut open)
        .collapsible(true)
        .resizable([true, true])
        .default_width(256.0)
        .min_width(215.0)
        .max_width(max_w)
        .max_height(max_h)
        .constrain_to(viewport_rect.shrink(6.0))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-6.0, 6.0))
        .frame(
            Frame::new()
                .fill(win_fill)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(10))
                .stroke(Stroke::new(1.0_f32, win_border))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 6],
                    blur: 18,
                    spread: 0,
                    color: Color32::from_black_alpha(if app.dark_mode { 115 } else { 55 }),
                }),
        )
        .show(ctx, |ui| {
            // Count pieces per layer
            let mut layer_counts: BTreeMap<String, usize> = BTreeMap::new();
            for piece in &app.layout.pieces {
                let lid = piece.layer_id.clone().unwrap_or_else(|| "layer-1".into());
                *layer_counts.entry(lid).or_insert(0) += 1;
            }

            let active_id = app.layout.active_layer_id.clone().unwrap_or_else(|| "layer-1".into());
            let mut delete_layer_id: Option<String> = None;
            let mut edit_layer_id: Option<String> = None;
            let mut move_sel_to_layer: Option<String> = None;
            let has_selection = !app.selected.is_empty();
            let total_layers = app.layout.layers.as_ref().map(|l| l.len()).unwrap_or(0);

            egui::ScrollArea::vertical()
                .id_salt("floating_layers_scroll")
                .auto_shrink([false, true])
                .max_height((viewport_rect.height() - 190.0).max(96.0))
                .show(ui, |ui| {
                    if let Some(layers) = &mut app.layout.layers {
                        let mut order: Vec<usize> = (0..layers.len()).collect();
                        order.sort_by(|&a, &b| {
                            layers[b]
                                .height_mm
                                .unwrap_or(0.0)
                                .partial_cmp(&layers[a].height_mm.unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        for idx in order {
                            let layer = &mut layers[idx];
                            let is_act = layer.id == active_id;
                            let swatch = parse_hex_color(layer.color.as_deref()).unwrap_or(p.primary);
                            let stroke = if is_act {
                                Stroke::new(1.4_f32, p.primary)
                            } else {
                                Stroke::new(1.0_f32, p.border_main)
                            };
                            let cnt = layer_counts.get(&layer.id).copied().unwrap_or(0);

                            Frame::new()
                                .fill(card_fill)
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::symmetric(8, 6))
                                .stroke(stroke)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        let (bar_r, _) = ui.allocate_exact_size(egui::vec2(4.0, 26.0), egui::Sense::hover());
                                        ui.painter().rect_filled(bar_r, 2.0, swatch);

                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 2.0;
                                            ui.horizontal(|ui| {
                                                if ui
                                                    .selectable_label(is_act, RichText::new(&layer.name).strong().size(12.0))
                                                    .clicked()
                                                {
                                                    app.layout.active_layer_id = Some(layer.id.clone());
                                                }
                                                if is_act {
                                                    Frame::new()
                                                        .fill(p.primary_hover)
                                                        .corner_radius(CornerRadius::same(3))
                                                        .inner_margin(Margin::symmetric(4, 1))
                                                        .show(ui, |ui| {
                                                            ui.label(
                                                                RichText::new(t.active_badge)
                                                                    .strong()
                                                                    .size(8.5)
                                                                    .color(Color32::WHITE),
                                                            );
                                                        });
                                                }
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:.0} mm  •  {} {}",
                                                        layer.height_mm.unwrap_or(0.0),
                                                        cnt,
                                                        t.status_pieces
                                                    ))
                                                    .size(10.0)
                                                    .color(p.text_muted),
                                                );
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    let sz = egui::vec2(20.0, 20.0);
                                                    if total_layers > 1
                                                        && icon_button_sized(
                                                            ui,
                                                            Icon::Trash,
                                                            sz,
                                                            false,
                                                            p.bg_input,
                                                            Color32::from_rgb(220, 38, 38),
                                                            Color32::from_rgb(220, 38, 38),
                                                            p.text_sub,
                                                            if app.lang == Lang::De { "Ebene loeschen" } else { "Delete layer" },
                                                        )
                                                        .clicked()
                                                    {
                                                        delete_layer_id = Some(layer.id.clone());
                                                    }
                                                    if ui
                                                        .small_button("✎")
                                                        .on_hover_text(if app.lang == Lang::De { "Ebene bearbeiten" } else { "Edit layer" })
                                                        .clicked()
                                                    {
                                                        edit_layer_id = Some(layer.id.clone());
                                                    }
                                                    if has_selection
                                                        && ui
                                                            .small_button("↳")
                                                            .on_hover_text(if app.lang == Lang::De { "Auswahl hierher verschieben" } else { "Move selection to this layer" })
                                                            .clicked()
                                                    {
                                                        move_sel_to_layer = Some(layer.id.clone());
                                                    }
                                                    let lock_icon = if layer.locked { Icon::Lock } else { Icon::Unlock };
                                                    if icon_button_sized(
                                                        ui,
                                                        lock_icon,
                                                        sz,
                                                        layer.locked,
                                                        p.bg_input,
                                                        p.primary_hover,
                                                        p.primary,
                                                        p.text_sub,
                                                        if app.lang == Lang::De { "Ebene sperren" } else { "Lock layer" },
                                                    )
                                                    .clicked()
                                                    {
                                                        layer.locked = !layer.locked;
                                                    }
                                                    let vis_icon = if layer.visible { Icon::Eye } else { Icon::EyeOff };
                                                    if icon_button_sized(
                                                        ui,
                                                        vis_icon,
                                                        sz,
                                                        false,
                                                        p.bg_input,
                                                        p.primary_hover,
                                                        p.primary,
                                                        if layer.visible { p.text_main } else { p.text_muted },
                                                        if app.lang == Lang::De { "Sichtbarkeit" } else { "Toggle visibility" },
                                                    )
                                                    .clicked()
                                                    {
                                                        layer.visible = !layer.visible;
                                                        visibility_or_layers_changed = true;
                                                    }
                                                });
                                            });
                                        });
                                    });
                                });
                        }
                    }
                });

            ui.add_space(6.0);
            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new(RichText::new(t.add_layer_full).strong().size(12.0).color(p.text_main))
                        .fill(card_fill)
                        .stroke(Stroke::new(1.0_f32, p.border_main))
                        .corner_radius(CornerRadius::same(7)),
                )
                .clicked()
            {
                app.open_layer_dialog_new();
            }

            if let Some(eid) = edit_layer_id {
                app.open_layer_dialog_edit(&eid);
            }
            if let Some(tid) = move_sel_to_layer {
                app.move_selected_to_layer(&tid);
            }

            ui.add_space(6.0);
            let orphaned_count = app.count_orphaned_items();
            if orphaned_count > 0 {
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(239, 68, 68, 30))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(8))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(239, 68, 68, 110)))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let warn_txt = if app.lang == Lang::De {
                            format!("{orphaned_count} Element(e) auf gelöschter Ebene")
                        } else {
                            format!("{orphaned_count} item(s) on deleted layer")
                        };
                        ui.label(RichText::new(warn_txt).strong().size(11.0).color(Color32::from_rgb(248, 113, 113)));
                        let btn_lbl = if app.lang == Lang::De {
                            "Aktiver Ebene zuweisen"
                        } else {
                            "Reassign to active layer"
                        };
                        if ui
                            .add(
                                egui::Button::new(RichText::new(btn_lbl).strong().size(11.0).color(Color32::WHITE))
                                    .fill(p.primary_hover)
                                    .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            app.reassign_orphaned_items();
                        }
                    });
                ui.add_space(6.0);
            }

            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new(RichText::new(t.heights_ramp_btn).strong().size(12.0).color(Color32::WHITE))
                        .fill(p.primary_hover)
                        .corner_radius(CornerRadius::same(7)),
                )
                .clicked()
            {
                app.show_height_dialog = true;
            }

            if let Some(del_id) = delete_layer_id {
                if app.count_items_on_layer(&del_id) > 0 {
                    app.confirm_delete_layer_id = Some(del_id);
                } else if app.safe_delete_layer(&del_id) {
                    visibility_or_layers_changed = true;
                }
            }
        });

    if !open && app.show_right_sidebar {
        app.show_right_sidebar = false;
        app.save_current_settings();
    }
    if visibility_or_layers_changed {
        app.mark_layout_dirty();
    }
}

/// Floating pill bar at bottom-center of canvas when 1+ tracks or models are selected (matches src/ui/selection-bar.ts).
pub fn draw_floating_selection_bar(ctx: &egui::Context, app: &mut RailbenchApp) {
    let t = tr(app.lang);
    let p = palette(app.dark_mode);

    // Subtle bottom-right zoom indicator pill (always visible, unobtrusive)
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

    if app.selected.is_empty() {
        return;
    }

    let sel_count = app.selected.len();
    let layers_list: Vec<(String, String)> = app
        .layout
        .layers
        .as_ref()
        .map(|ls| ls.iter().map(|l| (l.id.clone(), l.name.clone())).collect())
        .unwrap_or_else(|| vec![("layer-1".into(), "Ebene 0".into())]);

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
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Dup").strong().size(11.0).color(p.text_sub))
                                        .fill(p.bg_input)
                                        .stroke(Stroke::new(1.0_f32, p.border_main))
                                        .corner_radius(CornerRadius::same(8)),
                                )
                                .on_hover_text("Duplicate (Ctrl+D)")
                                .clicked()
                            {
                                app.duplicate_selected();
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
                            ui.label(RichText::new("[G]").strong().size(10.5).color(Color32::from_rgb(56, 189, 248)))
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
                                            Color32::from_rgb(239, 68, 68)
                                        } else {
                                            p.text_muted
                                        }),
                                )
                                .on_hover_text(format!(
                                    "L={:.0} mm · {:.0}→{:.0} mm (ΔZ={:.1} mm)",
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
                                    RichText::new(format!("({:.0}→{:.0})", layer_h + el.start, layer_h + el.end))
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
                                Color32::from_rgb(251, 191, 36)
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
                        if has_iso_any && toggle_pill_tri(ui, "⇄", false, false, Color32::from_rgb(185, 28, 28), if is_de { "Isoliertes Gleisende wechseln" } else { "Cycle isolated track end" }) {
                            app.cycle_selected_isolator_connector();
                        }
                        if toggle_pill_tri(ui, t.catenary, has_cat_all, has_cat_any, p.primary_hover, "Catenary (O)") {
                            app.toggle_selected_catenary();
                        }
                        if toggle_pill_tri(ui, "OL+", has_cat_feed_all, has_cat_feed_any, Color32::from_rgb(2, 132, 199), "Catenary Feeder (Shift+F)") {
                            app.toggle_selected_catenary_feeder();
                        }
                        let ol_iso_lbl = if sel_count == 1 && has_cat_iso_all {
                            let idx = sample.catenary_isolated_connector_index.unwrap_or(0) + 1;
                            let total = sample.connectors.len().max(2);
                            format!("OL/ {idx}/{total}")
                        } else {
                            "OL/".into()
                        };
                        if toggle_pill_tri(ui, &ol_iso_lbl, has_cat_iso_all, has_cat_iso_any, Color32::from_rgb(220, 38, 38), "Catenary Isolator (Shift+I)") {
                            app.toggle_selected_catenary_isolator();
                        }
                        if toggle_pill_tri(ui, "SIG", has_sig_all, has_sig_any, Color32::from_rgb(22, 163, 74), "Signal (Shift+S)") {
                            app.toggle_selected_signal();
                        }
                        if any_sig {
                            let side_lbl = match sig_side {
                                "left" => "L",
                                "both" => "L+R",
                                _ => "R",
                            };
                            if toggle_pill_tri(ui, side_lbl, true, true, Color32::from_rgb(15, 118, 110), "Cycle Signal Side") {
                                app.cycle_selected_signal_side();
                            }
                            let dir_lbl = match sig_dir {
                                "backward" => "<-",
                                "both" => "<->",
                                _ => "->",
                            };
                            if toggle_pill_tri(ui, dir_lbl, true, true, Color32::from_rgb(15, 118, 110), "Cycle Signal Direction") {
                                app.cycle_selected_signal_direction();
                            }
                        }
                        if any_turnout {
                            let lantern_lbl = if app.lang == Lang::De { "Laterne" } else { "Lantern" };
                            if toggle_pill_tri(ui, lantern_lbl, has_lantern_all, has_lantern_any, Color32::from_rgb(202, 138, 4), "Turnout Lantern (W)") {
                                app.toggle_selected_lantern();
                            }
                            if toggle_pill_tri(ui, "DEC", has_decoder_all, has_decoder_any, Color32::from_rgb(2, 132, 199), "Turnout Decoder (Shift+W / Shift+D)") {
                                app.toggle_selected_decoder();
                            }
                        }
                        if toggle_pill_tri(ui, t.bridge, has_bridge_all, has_bridge_any, p.primary_hover, "Bridge (B)") {
                            app.toggle_selected_bridge();
                        }
                        if toggle_pill_tri(ui, t.tunnel, has_tunnel_all, has_tunnel_any, p.primary_hover, "Tunnel (T)") {
                            app.toggle_selected_tunnel();
                        }

                        ui.separator();
                        if toggle_pill_tri(ui, "⇄", false, false, p.primary_hover, if is_de { "Spiegeln / Wenden (Alt+F)" } else { "Mirror / Flip (Alt+F)" }) {
                            app.flip_selected();
                        }
                        if sel_count >= 2 && toggle_pill_tri(ui, "Grp", false, false, p.primary_hover, if is_de { "Gruppieren / Loesen (G)" } else { "Group / Ungroup (G)" }) {
                            app.toggle_group_selected();
                        }
                        if icon_button_sized(ui, Icon::RotateCcw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_ccw).clicked() {
                            app.rotate_selected(-app.rotation_step);
                        }
                        if icon_button_sized(ui, Icon::RotateCw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_cw).clicked() {
                            app.rotate_selected(app.rotation_step);
                        }
                        if toggle_pill_tri(ui, "Dup", false, false, p.primary_hover, "Duplicate (Ctrl+D)") {
                            app.duplicate_selected();
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

