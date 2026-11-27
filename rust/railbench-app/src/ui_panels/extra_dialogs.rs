use super::*;

pub(super) fn draw_track_props_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let Some(piece_id) = app.show_track_props_piece_id.clone() else {
        return;
    };
    let Some(piece) = app.layout.pieces.iter().find(|p| p.id == piece_id).cloned() else {
        app.show_track_props_piece_id = None;
        return;
    };
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let entry = crate::catalog::find_entry(&app.catalogs, &piece.catalog_id);
    let layer = app
        .layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == piece.layer_id.as_deref().unwrap_or("layer-1")));
    let layer_name = layer
        .map(|l| l.name.as_str())
        .unwrap_or(if is_de { "Ebene 0" } else { "Layer 0" });
    let layer_h = layer.and_then(|l| l.height_mm).unwrap_or(0.0);
    let h0 = layer_h + piece.elevation.map_or(0.0, |e| e.start);
    let h1 = layer_h + piece.elevation.map_or(0.0, |e| e.end);
    let len_opt = piece.custom_length.or_else(|| entry.and_then(|e| e.length));
    let rad_opt = piece.custom_radius.or_else(|| entry.and_then(|e| e.radius));
    let ang_opt = piece.custom_angle.or_else(|| entry.and_then(|e| e.angle));
    let slope_pct = len_opt
        .filter(|&l| l > 1.0 && (h1 - h0).abs() > 0.1)
        .map(|l| ((h1 - h0).abs() / l) * 100.0)
        .unwrap_or(0.0);
    let open_ends = piece.connectors.iter().filter(|c| c.connected_to.is_none()).count();

    let mut open = true;
    let track_props_title = if is_de {
        "Gleiseigenschaften & Information"
    } else {
        "Track Properties & Info"
    };
    egui::Window::new(track_props_title)
        .id(egui::Id::new("railbench_track_props_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_width(380.0)
        .frame(crate::theme::railbench_window_frame(app.dark_mode))
        .show(ctx, |ui| {
            crate::theme::show_railbench_dialog_body(ui, app.dark_mode, track_props_title, Some(&mut open), 12, |ui| {
        egui::Grid::new("track_props_grid")
            .num_columns(2)
            .spacing([14.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                let mut row = |k: &str, v: String| {
                    ui.label(RichText::new(k).color(p.text_muted));
                    ui.label(RichText::new(v).strong().color(p.text_main));
                    ui.end_row();
                };
                row(if is_de { "Artikelnummer:" } else { "Article Number:" }, format!("#{}", piece.catalog_id));
                let name = entry
                    .map(|e| crate::catalog::localized_entry_name(e, is_de))
                    .unwrap_or("-");
                row(if is_de { "Bezeichnung:" } else { "Name / Type:" }, name.to_string());
                if let Some(desc) = entry
                    .map(|e| crate::catalog::localized_entry_description(e, is_de))
                    .filter(|d| !d.is_empty())
                {
                    row(if is_de { "Beschreibung:" } else { "Description:" }, desc.to_string());
                }
                row(
                    if is_de { "Kategorie:" } else { "Category:" },
                    entry
                        .map(|e| crate::catalog::localized_track_kind(&e.kind, is_de))
                        .unwrap_or("-")
                        .to_string(),
                );
                row(
                    if is_de { "Laenge:" } else { "Length:" },
                    len_opt.map(|l| format!("{l:.1} mm")).unwrap_or_else(|| "-".into()),
                );
                row(
                    "Radius:",
                    rad_opt.map(|r| format!("R = {r:.1} mm")).unwrap_or_else(|| "-".into()),
                );
                row(
                    if is_de { "Winkel:" } else { "Angle:" },
                    ang_opt.map(|a| format!("{a:.1}°")).unwrap_or_else(|| "-".into()),
                );
                row(
                    if is_de { "Spurweite:" } else { "Gauge:" },
                    format!("{:.1} mm ({})", entry.and_then(|e| e.gauge).unwrap_or(16.5), app.layout.scale),
                );
                row(if is_de { "Ebene:" } else { "Layer:" }, layer_name.to_string());
                let h_txt = if slope_pct > 0.05 {
                    format!("{h0:.0} mm -> {h1:.0} mm ({slope_pct:.1}%)")
                } else {
                    format!("{h0:.0} mm -> {h1:.0} mm")
                };
                row(if is_de { "Hoehe (Start/Ende):" } else { "Height (Start/End):" }, h_txt);
                row(
                    if is_de { "Offene Enden:" } else { "Open Ends:" },
                    format!("{open_ends} / {}", piece.connectors.len()),
                );
                row(
                    if is_de { "Stromeinspeisung:" } else { "Power Feeder:" },
                    if piece.power_feed == Some(true) { if is_de { "Aktiv" } else { "Active" } } else { "-" }.into(),
                );
                row(
                    if is_de { "Trennstelle:" } else { "Isolator Gap:" },
                    if piece.is_isolator == Some(true) {
                        format!("Pin {}", piece.isolated_connector_index.unwrap_or(0) + 1)
                    } else {
                        "-".into()
                    },
                );
                row(
                    if is_de { "Oberleitung:" } else { "Catenary:" },
                    if piece.has_catenary == Some(true) { if is_de { "Aktiv" } else { "Active" } } else { "-" }.into(),
                );
                row(
                    "Signal:",
                    if piece.has_signal == Some(true) {
                        let side_lbl = match piece.signal_side.as_deref().unwrap_or("right") {
                            "left" => if is_de { "Links" } else { "Left" },
                            _ => if is_de { "Rechts" } else { "Right" },
                        };
                        let dir_lbl = match piece.signal_direction.as_deref().unwrap_or("forward") {
                            "backward" => if is_de { "Rückwärts" } else { "Backward" },
                            "both" => if is_de { "Beide" } else { "Both" },
                            _ => if is_de { "Vorwärts" } else { "Forward" },
                        };
                        format!("{side_lbl}, {dir_lbl}")
                    } else {
                        "-".into()
                    },
                );
                row(
                    if is_de { "Bruecke / Tunnel:" } else { "Bridge / Tunnel:" },
                    if piece.has_bridge == Some(true) {
                        if is_de { "Bruecke" } else { "Bridge" }
                    } else if piece.has_tunnel == Some(true) {
                        "Tunnel"
                    } else {
                        "-"
                    }
                    .into(),
                );
                if let Some(ref gid) = piece.group_id {
                    row(if is_de { "Gruppe:" } else { "Group:" }, gid.clone());
                }
            });
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(if is_de { "Schliessen" } else { "Close" }).clicked() {
                app.show_track_props_piece_id = None;
            }
        });
            });
    });
    if !open {
        app.show_track_props_piece_id = None;
    }
}

pub(super) fn draw_replace_split_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let Some(piece_id) = app.show_replace_track_piece_id.clone() else {
        return;
    };
    let Some(piece) = app.layout.pieces.iter().find(|p| p.id == piece_id).cloned() else {
        app.show_replace_track_piece_id = None;
        return;
    };
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let entry = crate::catalog::find_entry(&app.catalogs, &piece.catalog_id).cloned();
    let is_flex = crate::catalog::is_flex_id(&app.catalogs, &piece.catalog_id);
    let can_split = is_flex || entry.as_ref().map_or(false, |e| e.kind == "straight");

    let mut open = true;
    let mut do_split = false;
    let mut chosen_replace: Option<String> = None;

    let replace_title = if is_de {
        "Gleis teilen oder Typ ersetzen"
    } else {
        "Split Track or Replace Type"
    };
    egui::Window::new(replace_title)
        .id(egui::Id::new("railbench_replace_split_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_width(440.0)
        .frame(crate::theme::railbench_window_frame(app.dark_mode))
        .show(ctx, |ui| {
            crate::theme::show_railbench_dialog_body(ui, app.dark_mode, replace_title, Some(&mut open), 12, |ui| {
        if can_split {
            Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::same(8))
                .stroke(Stroke::new(1.0_f32, p.border_main))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.strong(if is_de {
                        "Gleis in zwei Haelften teilen"
                    } else {
                        "Split Track into Halves"
                    });
                    ui.small(if is_flex {
                        if is_de {
                            "Teilt das Flexgleis mittig in zwei verbundene Stuecke."
                        } else {
                            "Cuts the flextrack in half into two connected pieces."
                        }
                    } else if is_de {
                        "Ersetzt das gerade Gleis durch zwei passende Halb-Gleise aus dem Katalog."
                    } else {
                        "Replaces this straight track with two matching half-length catalog tracks."
                    });
                    ui.add_space(4.0);
                    if ui
                        .button(if is_de { "✂ Gleis jetzt teilen" } else { "✂ Split Track Now" })
                        .clicked()
                    {
                        do_split = true;
                    }
                });
            ui.add_space(6.0);
        }

        ui.strong(if is_de {
            "Durch einen anderen Katalog-Typ ersetzen:"
        } else {
            "Replace with another catalog piece:"
        });
        ui.add(
            egui::TextEdit::singleline(&mut app.replace_track_search)
                .hint_text(if is_de { "Suchen (#Nr / Name)..." } else { "Search (#id / name)..." })
                .desired_width(ui.available_width()),
        );
        ui.add_space(4.0);

        let owner_entries = app
            .catalogs
            .iter()
            .find(|c| c.entries.iter().any(|e| e.id == piece.catalog_id))
            .or_else(|| app.catalogs.get(app.selected_catalog_idx))
            .map(|c| c.entries.as_slice())
            .unwrap_or(&[]);
        let cur_kind = entry.as_ref().map(|e| e.kind.as_str()).unwrap_or("");
        let q = app.replace_track_search.trim().to_ascii_lowercase();

        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
            let mut count = 0usize;
            for cand in owner_entries {
                if cand.id == piece.catalog_id {
                    continue;
                }
                let kind_match = q.is_empty()
                    && ((cur_kind == "straight" && cand.kind == "straight")
                        || (cur_kind.contains("curve") && cand.kind.contains("curve"))
                        || (cur_kind.contains("turnout") && cand.kind.contains("turnout"))
                        || cur_kind == cand.kind);
                let text_match = !q.is_empty()
                    && (cand.id.to_ascii_lowercase().contains(&q)
                        || cand.name.to_ascii_lowercase().contains(&q)
                        || cand.name_en.as_deref().unwrap_or("").to_ascii_lowercase().contains(&q)
                        || cand.description_en.as_deref().unwrap_or("").to_ascii_lowercase().contains(&q));
                if !kind_match && !text_match {
                    continue;
                }
                let name = if !is_de {
                    cand.name_en.as_deref().unwrap_or(&cand.name)
                } else {
                    cand.name.as_str()
                };
                let spec = cand
                    .length
                    .map(|l| format!("{l:.0} mm"))
                    .or_else(|| cand.radius.map(|r| format!("R{r:.0}")))
                    .unwrap_or_default();
                if ui
                    .button(format!("#{} · {} ({spec})", cand.id, name))
                    .clicked()
                {
                    chosen_replace = Some(cand.id.clone());
                }
                count += 1;
                if count >= 40 {
                    break;
                }
            }
            if count == 0 {
                ui.weak(if is_de {
                    "Keine passenden Ersatzgleise gefunden."
                } else {
                    "No matching replacement tracks found."
                });
            }
        });
            });
    });

    if do_split {
        app.split_track(&piece_id);
        app.show_replace_track_piece_id = None;
    } else if let Some(new_cid) = chosen_replace {
        app.replace_track(&piece_id, &new_cid);
        app.show_replace_track_piece_id = None;
    } else if !open {
        app.show_replace_track_piece_id = None;
    }
}

fn draw_layer_section_card(
    ui: &mut egui::Ui,
    p: crate::theme::RailbenchPalette,
    dark: bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let sec_fill = if dark {
        Color32::from_rgb(17, 24, 39)
    } else {
        p.bg_sidebar
    };
    Frame::new()
        .fill(sec_fill)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 12))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui);
        });
}

pub(super) fn draw_layer_edit_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let surface = if app.dark_mode {
        Color32::from_rgb(21, 31, 50)
    } else {
        Color32::from_rgb(241, 245, 249)
    };
    let is_edit = app.layer_dialog_edit_id.is_some();
    let mut open = app.show_layer_dialog;
    let mut submit = false;
    let screen_rect = ctx.screen_rect();

    // Handle Escape / Enter keyboard shortcuts just like Electron's layer-dialog
    let (esc_pressed, enter_pressed) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::Escape),
            i.key_pressed(egui::Key::Enter),
        )
    });
    if esc_pressed {
        if app.layer_dialog_custom_color_open {
            app.layer_dialog_custom_color_open = false;
        } else {
            open = false;
        }
    } else if enter_pressed && !app.layer_dialog_custom_color_open {
        submit = true;
    }

    // Translucent backdrop (.rb-modal-overlay)
    egui::Area::new(egui::Id::new("railbench_layer_backdrop"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            let (bg_rect, bg_resp) =
                ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
            ui.painter().rect_filled(
                bg_rect,
                0.0,
                Color32::from_rgba_unmultiplied(15, 23, 42, if app.dark_mode { 175 } else { 150 }),
            );
            if bg_resp.clicked() {
                open = false;
            }
        });

    let layer_title = if is_edit {
        if is_de { "Ebene bearbeiten" } else { "Edit Layer" }
    } else if is_de {
        "Neue Ebene anlegen"
    } else {
        "Create New Layer"
    };

    let modal_w = 460.0_f32.min((screen_rect.width() * 0.94).max(340.0));
    let max_body_h = (screen_rect.height() * 0.76).max(240.0);
    let modal_frame = Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::ZERO)
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .shadow(egui::Shadow {
            offset: [0, 14],
            blur: 32,
            spread: 0,
            color: Color32::from_black_alpha(if app.dark_mode { 165 } else { 85 }),
        });

    egui::Window::new(layer_title)
        .id(egui::Id::new("railbench_layer_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(modal_w)
        .min_width(modal_w)
        .max_width(modal_w)
        .frame(modal_frame)
        .show(ctx, |ui| {
            ui.set_width(modal_w);
            ui.spacing_mut().item_spacing.y = 0.0;

            // --- Modal Header (.rb-modal-header) ---
            let hdr_resp = Frame::new()
                .fill(surface)
                .corner_radius(CornerRadius {
                    nw: 10,
                    ne: 10,
                    sw: 0,
                    se: 0,
                })
                .inner_margin(Margin::symmetric(18, 11))
                .show(ui, |ui| {
                    ui.set_width(modal_w - 36.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(layer_title)
                                .strong()
                                .size(15.0)
                                .color(p.text_main),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (close_r, close_resp) =
                                ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
                            if close_resp.hovered() {
                                ui.painter()
                                    .rect_filled(close_r, CornerRadius::same(4), p.border_main);
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            crate::icons::paint_icon(
                                ui.painter(),
                                close_r.shrink(7.0),
                                crate::icons::Icon::WinClose,
                                if close_resp.hovered() {
                                    p.text_main
                                } else {
                                    p.text_muted
                                },
                            );
                            if close_resp
                                .on_hover_text(if is_de { "Schliessen (Esc)" } else { "Close (Esc)" })
                                .clicked()
                            {
                                open = false;
                            }
                        });
                    });
                });
            let hr = hdr_resp.response.rect;
            ui.painter()
                .hline(hr.x_range(), hr.bottom(), Stroke::new(1.0_f32, p.border_main));

            // --- Scrollable Modal Body (.layer-modal-body) ---
            egui::ScrollArea::vertical()
                .id_salt("railbench_layer_dialog_body")
                .max_height(max_body_h)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    Frame::new()
                        .inner_margin(Margin::symmetric(20, 16))
                        .show(ui, |ui| {
                            ui.set_width(modal_w - 40.0);
                            ui.spacing_mut().item_spacing.y = 12.0;

                            // 1. Layer Name (.layer-dialog-section)
                            draw_layer_section_card(ui, p, app.dark_mode, |ui| {
                                ui.spacing_mut().item_spacing.y = 6.0;
                                ui.label(
                                    RichText::new(if is_de {
                                        "Ebenen-Bezeichnung:"
                                    } else {
                                        "Layer Name:"
                                    })
                                    .strong()
                                    .size(12.0)
                                    .color(p.text_main),
                                );
                                let name_out = egui::TextEdit::singleline(&mut app.layer_dialog_name)
                                    .id_source("railbench_layer_name_input")
                                    .char_limit(50)
                                    .desired_width(ui.available_width())
                                    .margin(Margin::symmetric(10, 7))
                                    .hint_text(if is_de {
                                        "z.B. Schattenbahnhof, Bergstrecke..."
                                    } else {
                                        "e.g. Staging Yard, Mountain Level..."
                                    })
                                    .show(ui);
                                if app.layer_dialog_focus_name {
                                    name_out.response.request_focus();
                                    let len = app.layer_dialog_name.chars().count();
                                    let mut state = name_out.state;
                                    state.cursor.set_char_range(Some(
                                        egui::text::CCursorRange::two(
                                            egui::text::CCursor::new(0),
                                            egui::text::CCursor::new(len),
                                        ),
                                    ));
                                    state.store(ctx, name_out.response.id);
                                    app.layer_dialog_focus_name = false;
                                }
                                if name_out.response.lost_focus()
                                    && ctx.input(|i| i.key_pressed(egui::Key::Enter))
                                    && !app.layer_dialog_custom_color_open
                                {
                                    submit = true;
                                }
                            });

                            // 2. Layer Elevation (.layer-dialog-section)
                            draw_layer_section_card(ui, p, app.dark_mode, |ui| {
                                ui.spacing_mut().item_spacing.y = 6.0;
                                ui.label(
                                    RichText::new(if is_de {
                                        "Ebenenhoehe:"
                                    } else {
                                        "Layer Height:"
                                    })
                                    .strong()
                                    .size(12.0)
                                    .color(p.text_main),
                                );
                                Frame::new()
                                    .fill(p.bg_card)
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(Margin::symmetric(10, 6))
                                    .stroke(Stroke::new(1.0_f32, p.border_main))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            let dv_w = (ui.available_width() - 34.0).max(80.0);
                                            ui.add_sized(
                                                [dv_w, 24.0],
                                                egui::DragValue::new(&mut app.layer_dialog_height_mm)
                                                    .speed(1.0)
                                                    .range(-5000.0..=5000.0)
                                                    .max_decimals(1),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        RichText::new("mm")
                                                            .strong()
                                                            .size(13.0)
                                                            .color(p.text_muted),
                                                    );
                                                },
                                            );
                                        });
                                    });
                            });

                            // 3. Layer Color (.layer-dialog-section + 10 swatches + Custom Color Picker)
                            draw_layer_section_card(ui, p, app.dark_mode, |ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;
                                ui.label(
                                    RichText::new(if is_de {
                                        "Farbe der Ebene:"
                                    } else {
                                        "Layer Color:"
                                    })
                                    .strong()
                                    .size(12.0)
                                    .color(p.text_main),
                                );
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                                    for &hex in &RailbenchApp::LAYER_COLOR_PALETTE {
                                        let col = crate::canvas2d::parse_hex_color(Some(hex))
                                            .unwrap_or(p.primary);
                                        let active = app.layer_dialog_color.eq_ignore_ascii_case(hex);
                                        let (rect, resp) = ui.allocate_exact_size(
                                            egui::vec2(28.0, 28.0),
                                            egui::Sense::click(),
                                        );
                                        let hovered = resp.hovered();
                                        let c = rect.center();
                                        let radius = if hovered { 14.5 } else { 13.0 };
                                        if hovered {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                        if active {
                                            ui.painter().circle_stroke(
                                                c,
                                                radius + 2.0,
                                                Stroke::new(2.0_f32, p.primary),
                                            );
                                        }
                                        ui.painter().circle_filled(c, radius, col);
                                        ui.painter().circle_stroke(
                                            c,
                                            radius,
                                            Stroke::new(
                                                if active { 2.0_f32 } else { 1.0_f32 },
                                                if active {
                                                    Color32::WHITE
                                                } else {
                                                    Color32::from_black_alpha(45)
                                                },
                                            ),
                                        );
                                        if active {
                                            let s = Color32::from_black_alpha(150);
                                            let pts1 = [
                                                c + egui::vec2(-5.2, 0.2),
                                                c + egui::vec2(-1.4, 4.0),
                                            ];
                                            let pts2 = [
                                                c + egui::vec2(-1.4, 4.0),
                                                c + egui::vec2(5.4, -3.6),
                                            ];
                                            ui.painter().line_segment(
                                                [pts1[0] + egui::vec2(0.0, 0.8), pts1[1] + egui::vec2(0.0, 0.8)],
                                                Stroke::new(2.6_f32, s),
                                            );
                                            ui.painter().line_segment(
                                                [pts2[0] + egui::vec2(0.0, 0.8), pts2[1] + egui::vec2(0.0, 0.8)],
                                                Stroke::new(2.6_f32, s),
                                            );
                                            ui.painter().line_segment(pts1, Stroke::new(2.2_f32, Color32::WHITE));
                                            ui.painter().line_segment(pts2, Stroke::new(2.2_f32, Color32::WHITE));
                                        }
                                        if resp.on_hover_text(hex).clicked() {
                                            app.layer_dialog_color = hex.into();
                                            app.layer_dialog_custom_color_open = false;
                                        }
                                    }

                                    // 11th button: Custom Color Picker (.layer-custom-color-picker)
                                    let is_custom = !RailbenchApp::LAYER_COLOR_PALETTE
                                        .iter()
                                        .any(|h| h.eq_ignore_ascii_case(&app.layer_dialog_color));
                                    let custom_col =
                                        crate::canvas2d::parse_hex_color(Some(&app.layer_dialog_color))
                                            .unwrap_or(p.primary);
                                    let (pick_r, pick_resp) = ui.allocate_exact_size(
                                        egui::vec2(28.0, 28.0),
                                        egui::Sense::click(),
                                    );
                                    let pc = pick_r.center();
                                    let pr = if pick_resp.hovered() { 14.5 } else { 13.0 };
                                    if pick_resp.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                    if is_custom || app.layer_dialog_custom_color_open {
                                        ui.painter().circle_stroke(
                                            pc,
                                            pr + 2.0,
                                            Stroke::new(2.0_f32, p.primary),
                                        );
                                    }
                                    ui.painter().circle_filled(
                                        pc,
                                        pr,
                                        if is_custom { custom_col } else { p.bg_card },
                                    );
                                    ui.painter().circle_stroke(
                                        pc,
                                        pr,
                                        Stroke::new(1.2_f32, p.border_main),
                                    );
                                    crate::icons::paint_icon(
                                        ui.painter(),
                                        pick_r.shrink(7.0),
                                        crate::icons::Icon::Palette,
                                        if is_custom { Color32::WHITE } else { p.text_muted },
                                    );
                                    if pick_resp
                                        .on_hover_text(if is_de {
                                            "Eigene Farbe waehlen..."
                                        } else {
                                            "Pick custom color..."
                                        })
                                        .clicked()
                                    {
                                        app.layer_dialog_custom_color_open =
                                            !app.layer_dialog_custom_color_open;
                                    }
                                });

                                if app.layer_dialog_custom_color_open {
                                    let mut cur_col =
                                        crate::canvas2d::parse_hex_color(Some(&app.layer_dialog_color))
                                            .unwrap_or(p.primary);
                                    Frame::new()
                                        .fill(p.bg_card)
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(Margin::same(10))
                                        .stroke(Stroke::new(1.0_f32, p.border_main))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            if egui::color_picker::color_picker_color32(
                                                ui,
                                                &mut cur_col,
                                                egui::color_picker::Alpha::Opaque,
                                            ) {
                                                app.layer_dialog_color = format!(
                                                    "#{:02x}{:02x}{:02x}",
                                                    cur_col.r(),
                                                    cur_col.g(),
                                                    cur_col.b()
                                                );
                                            }
                                            ui.add_space(4.0);
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Hex: {}",
                                                        app.layer_dialog_color
                                                    ))
                                                    .small()
                                                    .color(p.text_muted),
                                                );
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        if ui
                                                            .small_button(if is_de {
                                                                "Fertig"
                                                            } else {
                                                                "Done"
                                                            })
                                                            .clicked()
                                                        {
                                                            app.layer_dialog_custom_color_open =
                                                                false;
                                                        }
                                                    },
                                                );
                                            });
                                        });
                                }
                            });

                            // 4. Wood Plate / Subroadbed (.layer-dialog-section + 3 radio cards)
                            draw_layer_section_card(ui, p, app.dark_mode, |ui| {
                                ui.spacing_mut().item_spacing.y = 6.0;
                                ui.label(
                                    RichText::new(if is_de {
                                        "Trassenplatte / Unterbau:"
                                    } else {
                                        "Baseboard Deck / Subroadbed:"
                                    })
                                    .strong()
                                    .size(12.0)
                                    .color(p.text_main),
                                );

                                let options = [
                                    (
                                        "none",
                                        if is_de { "Keine Holzplatte" } else { "No Wood Plate" },
                                        if is_de {
                                            "Keine separate Holzplatte fuer diese Ebene erzeugen"
                                        } else {
                                            "Do not generate a wood plate under this layer"
                                        },
                                    ),
                                    (
                                        "table",
                                        if is_de {
                                            "Passend zum Tisch (Volle Platte)"
                                        } else {
                                            "Fit to Table (Full Deck)"
                                        },
                                        if is_de {
                                            "Vollflaechige Zwischenebene exakt im Grundriss der Tischplatte"
                                        } else {
                                            "Full deck matching the table baseboard contour at this layer height"
                                        },
                                    ),
                                    (
                                        "tight",
                                        if is_de {
                                            "Nur unter Gleisen & Gebaeuden (Trassenbrett)"
                                        } else {
                                            "Only Fit to Tracks & Buildings"
                                        },
                                        if is_de {
                                            "Passgenaues Trassenbrett nur unter Gleisen und Gebaeuden dieser Ebene"
                                        } else {
                                            "Tailored roadbed boards only under the tracks and buildings of this layer"
                                        },
                                    ),
                                ];

                                for (mode, title_txt, desc_txt) in options {
                                    let active = app.layer_dialog_wood_mode == mode;
                                    let choice_fill = if active {
                                        Color32::from_rgba_unmultiplied(
                                            p.primary.r(),
                                            p.primary.g(),
                                            p.primary.b(),
                                            if app.dark_mode { 36 } else { 22 },
                                        )
                                    } else {
                                        p.bg_card
                                    };
                                    let choice_stroke = if active {
                                        Stroke::new(1.5_f32, p.primary)
                                    } else {
                                        Stroke::new(1.0_f32, p.border_main)
                                    };
                                    let card_resp = Frame::new()
                                        .fill(choice_fill)
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(Margin::symmetric(12, 8))
                                        .stroke(choice_stroke)
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.horizontal_top(|ui| {
                                                ui.spacing_mut().item_spacing.x = 10.0;
                                                let (dot_r, _) = ui.allocate_exact_size(
                                                    egui::vec2(16.0, 18.0),
                                                    egui::Sense::hover(),
                                                );
                                                let dc = dot_r.center();
                                                ui.painter().circle_stroke(
                                                    dc,
                                                    6.5,
                                                    Stroke::new(
                                                        1.5_f32,
                                                        if active { p.primary } else { p.text_muted },
                                                    ),
                                                );
                                                if active {
                                                    ui.painter().circle_filled(dc, 3.4, p.primary);
                                                }
                                                ui.vertical(|ui| {
                                                    ui.spacing_mut().item_spacing.y = 2.0;
                                                    ui.label(
                                                        RichText::new(title_txt)
                                                            .strong()
                                                            .size(13.0)
                                                            .color(p.text_main),
                                                    );
                                                    ui.label(
                                                        RichText::new(desc_txt)
                                                            .size(11.0)
                                                            .color(p.text_muted),
                                                    );
                                                });
                                            });
                                        });
                                    let interact_resp = ui.interact(
                                        card_resp.response.rect,
                                        ui.id().with(("layer_woodplate_choice", mode)),
                                        egui::Sense::click(),
                                    );
                                    if interact_resp.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                    if interact_resp.clicked() {
                                        app.layer_dialog_wood_mode = mode.into();
                                    }
                                }

                                if app.layer_dialog_wood_mode != "none" {
                                    ui.add_space(2.0);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(if is_de {
                                                "Plattenstaerke:"
                                            } else {
                                                "Thickness:"
                                            })
                                            .size(12.0)
                                            .color(p.text_muted),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                Frame::new()
                                                    .fill(p.bg_card)
                                                    .corner_radius(CornerRadius::same(6))
                                                    .inner_margin(Margin::symmetric(8, 4))
                                                    .stroke(Stroke::new(1.0_f32, p.border_main))
                                                    .show(ui, |ui| {
                                                        ui.set_width(104.0);
                                                        ui.horizontal(|ui| {
                                                            ui.add_sized(
                                                                [68.0, 20.0],
                                                                egui::DragValue::new(
                                                                    &mut app.layer_dialog_wood_thickness,
                                                                )
                                                                .range(2.0..=50.0)
                                                                .speed(1.0)
                                                                .max_decimals(1),
                                                            );
                                                            ui.label(
                                                                RichText::new("mm")
                                                                    .size(12.0)
                                                                    .color(p.text_muted),
                                                            );
                                                        });
                                                    });
                                            },
                                        );
                                    });
                                }
                            });
                        });
                });

            // --- Modal Footer (.rb-modal-footer) ---
            let footer_sep = ui.available_rect_before_wrap();
            ui.painter().hline(
                footer_sep.x_range(),
                footer_sep.top(),
                Stroke::new(1.0_f32, p.border_main),
            );
            Frame::new()
                .fill(surface)
                .corner_radius(CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: 10,
                    se: 10,
                })
                .inner_margin(Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.set_width(modal_w - 36.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let submit_lbl = if is_edit {
                            if is_de {
                                "Aenderungen speichern"
                            } else {
                                "Save Changes"
                            }
                        } else if is_de {
                            "Ebene anlegen"
                        } else {
                            "Create Layer"
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(submit_lbl)
                                        .strong()
                                        .size(12.5)
                                        .color(Color32::WHITE),
                                )
                                .fill(p.primary_hover)
                                .stroke(Stroke::new(1.0_f32, p.primary))
                                .min_size(egui::vec2(124.0, 32.0))
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            submit = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(if is_de { "Abbrechen" } else { "Cancel" })
                                        .size(12.5)
                                        .color(p.text_main),
                                )
                                .fill(p.bg_card)
                                .stroke(Stroke::new(1.0_f32, p.border_main))
                                .min_size(egui::vec2(96.0, 32.0))
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            open = false;
                        }
                    });
                });
        });

    if submit {
        app.apply_layer_dialog();
    } else if !open {
        app.layer_dialog_custom_color_open = false;
        app.layer_dialog_focus_name = false;
        app.show_layer_dialog = false;
    }
}

pub(super) fn draw_confirm_delete_layer_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let Some(del_id) = app.confirm_delete_layer_id.clone() else {
        return;
    };
    let is_de = app.lang == Lang::De;
    let cnt = app.count_items_on_layer(&del_id);
    let layer_name = app
        .layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == del_id))
        .map(|l| l.name.clone())
        .unwrap_or_else(|| del_id.clone());

    let mut open = true;
    let mut confirm = false;
    let del_title = if is_de { "Ebene loeschen?" } else { "Delete Layer?" };
    egui::Window::new(del_title)
        .id(egui::Id::new("railbench_confirm_delete_layer_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .frame(crate::theme::railbench_window_frame(app.dark_mode))
        .show(ctx, |ui| {
            crate::theme::show_railbench_dialog_body(ui, app.dark_mode, del_title, Some(&mut open), 12, |ui| {
            ui.label(if is_de {
                format!(
                    "Auf Ebene \"{layer_name}\" befinden sich noch {cnt} Element(e). Beim Loeschen werden sie sicher auf die Standard-Ebene verschoben."
                )
            } else {
                format!(
                    "Layer \"{layer_name}\" contains {cnt} item(s). Deleting it will safely move those items to the default layer."
                )
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(if is_de { "Abbrechen" } else { "Cancel" }).clicked() {
                    app.confirm_delete_layer_id = None;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new(if is_de { "Ebene loeschen" } else { "Delete Layer" }).color(Color32::WHITE))
                                .fill(Color32::from_rgb(220, 38, 38)),
                        )
                        .clicked()
                    {
                        confirm = true;
                    }
                });
            });
            });
        });

    if confirm {
        app.safe_delete_layer(&del_id);
        app.confirm_delete_layer_id = None;
    } else if !open {
        app.confirm_delete_layer_id = None;
    }
}

pub(super) fn draw_confirm_unsaved_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let Some(action) = app.pending_confirm_action.clone() else {
        return;
    };
    let is_de = app.lang == Lang::De;
    let mut choice: Option<u8> = None;

    let unsaved_title = if is_de {
        "Ungesicherte Aenderungen?"
    } else {
        "Unsaved Changes?"
    };
    egui::Window::new(unsaved_title)
        .id(egui::Id::new("railbench_confirm_unsaved_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(370.0)
        .frame(crate::theme::railbench_window_frame(app.dark_mode))
        .show(ctx, |ui| {
            crate::theme::show_railbench_dialog_body(ui, app.dark_mode, unsaved_title, None, 12, |ui| {
        ui.label(if is_de {
            format!("Der Plan \"{}\" hat ungesicherte Aenderungen. Moechten Sie vor dem Fortfahren speichern?", app.layout.name)
        } else {
            format!("Layout \"{}\" has unsaved changes. Save before continuing?", app.layout.name)
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button(if is_de { "Abbrechen" } else { "Cancel" }).clicked() {
                choice = Some(0);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new(if is_de { "Speichern & Weiter" } else { "Save & Continue" }).color(Color32::WHITE))
                            .fill(Color32::from_rgb(2, 132, 199)),
                    )
                    .clicked()
                {
                    choice = Some(2);
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new(if is_de { "Verwerfen" } else { "Discard" }).color(Color32::WHITE))
                            .fill(Color32::from_rgb(220, 38, 38)),
                    )
                    .clicked()
                {
                    choice = Some(1);
                }
            });
        });
            });
    });

    if let Some(c) = choice {
        app.pending_confirm_action = None;
        if c == 0 {
            return;
        }
        if c == 2 && !app.save_layout_dialog() {
            return;
        }
        match action {
            crate::PendingConfirmAction::New => app.new_empty_layout(),
            crate::PendingConfirmAction::Open => app.open_layout_dialog(),
            crate::PendingConfirmAction::Demo(path) => app.open_demo_layout(&path, true),
            crate::PendingConfirmAction::Quit => {
                app.allow_close_now = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

pub(super) fn draw_autosave_recovery_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    if !app.show_autosave_recovery {
        return;
    }
    let is_de = app.lang == Lang::De;
    let mut act: Option<bool> = None;

    let autosave_title = if is_de {
        "Automatische Sicherung gefunden"
    } else {
        "Unsaved Session Recovery"
    };
    egui::Window::new(autosave_title)
        .id(egui::Id::new("railbench_autosave_recovery_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(380.0)
        .frame(crate::theme::railbench_window_frame(app.dark_mode))
        .show(ctx, |ui| {
            crate::theme::show_railbench_dialog_body(ui, app.dark_mode, autosave_title, None, 12, |ui| {
        ui.label(if is_de {
            "Es wurde eine automatisch gespeicherte Sitzung gefunden. Moechten Sie diese wiederherstellen?"
        } else {
            "An autosaved backup from a previous session was found. Restore it now?"
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(if is_de { "Verwerfen" } else { "Discard" }).clicked() {
                act = Some(false);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new(if is_de { "Wiederherstellen" } else { "Restore Session" }).color(Color32::WHITE))
                            .fill(Color32::from_rgb(22, 163, 74)),
                    )
                    .clicked()
                {
                    act = Some(true);
                }
            });
        });
            });
    });

    match act {
        Some(true) => app.restore_autosave(),
        Some(false) => app.discard_autosave(),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_dialog_palette_open_and_apply_matches_electron_defaults() {
        assert_eq!(RailbenchApp::LAYER_COLOR_PALETTE.len(), 10);
        assert_eq!(RailbenchApp::LAYER_COLOR_PALETTE[0], "#38bdf8");
        assert_eq!(RailbenchApp::LAYER_COLOR_PALETTE[9], "#94a3b8");

        let mut app = RailbenchApp::default();
        app.lang = Lang::En;
        app.open_layer_dialog_new();
        assert!(app.show_layer_dialog);
        assert!(app.layer_dialog_focus_name);
        assert!(!app.layer_dialog_custom_color_open);
        assert!(app.layer_dialog_edit_id.is_none());
        assert_eq!(app.layer_dialog_wood_mode, "none");
        assert!((app.layer_dialog_wood_thickness - 10.0).abs() < 1e-6);

        let before_count = app.layout.layers.as_ref().map(|l| l.len()).unwrap_or(0);
        app.layer_dialog_name = "Mountain Line".into();
        app.layer_dialog_height_mm = 160.26;
        app.layer_dialog_color = "#12abef".into();
        app.layer_dialog_wood_mode = "tight".into();
        app.layer_dialog_wood_thickness = 75.0; // clamps to 50.0
        app.apply_layer_dialog();

        assert!(!app.show_layer_dialog);
        assert!(!app.layer_dialog_focus_name);
        assert!(!app.layer_dialog_custom_color_open);
        let layers = app.layout.layers.as_ref().expect("layers present");
        assert_eq!(layers.len(), before_count + 1);
        let created = layers.last().unwrap();
        assert_eq!(created.name, "Mountain Line");
        assert_eq!(created.height_mm, Some(160.3));
        assert_eq!(created.color.as_deref(), Some("#12abef"));
        assert_eq!(created.wood_plate_mode.as_deref(), Some("tight"));
        assert_eq!(created.wood_plate_thickness_mm, Some(50.0));
        let created_id = created.id.clone();

        // Open next new layer -> height should auto-offset +80 mm above 160.3 mm
        app.open_layer_dialog_new();
        assert!((app.layer_dialog_height_mm - 240.3).abs() < 1e-6);

        // Editing with blank name preserves existing layer name
        app.open_layer_dialog_edit(&created_id);
        app.layer_dialog_name = "   ".into();
        app.layer_dialog_wood_mode = "table".into();
        app.layer_dialog_wood_thickness = 1.0; // clamps to 2.0
        app.apply_layer_dialog();

        let updated = app
            .layout
            .layers
            .as_ref()
            .unwrap()
            .iter()
            .find(|l| l.id == created_id)
            .unwrap();
        assert_eq!(updated.name, "Mountain Line");
        assert_eq!(updated.wood_plate_mode.as_deref(), Some("table"));
        assert_eq!(updated.wood_plate_thickness_mm, Some(2.0));
    }

    #[test]
    fn layer_dialog_renders_headless_create_and_edit_and_closes_on_escape() {
        let mut app = RailbenchApp::default();
        let ctx = egui::Context::default();
        app.open_layer_dialog_new();
        app.dark_mode = true;
        app.lang = Lang::De;

        let run_frame = |ctx: &egui::Context, app: &mut RailbenchApp, keys: Vec<egui::Key>| {
            let events = keys
                .into_iter()
                .map(|key| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
                .collect();
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 800.0),
                )),
                events,
                ..Default::default()
            };
            let _ = ctx.run(raw, |ctx| {
                if app.show_layer_dialog {
                    draw_layer_edit_dialog(ctx, app);
                }
            });
        };

        // 1. Create mode render clears focus flag without panicking
        run_frame(&ctx, &mut app, vec![]);
        assert!(app.show_layer_dialog);
        assert!(!app.layer_dialog_focus_name);

        // 2. Edit mode in light theme with tight wood plate + custom color picker open
        app.dark_mode = false;
        app.lang = Lang::En;
        app.open_layer_dialog_edit("layer-1");
        app.layer_dialog_wood_mode = "tight".into();
        app.layer_dialog_custom_color_open = true;
        run_frame(&ctx, &mut app, vec![]);
        assert!(app.show_layer_dialog);
        assert!(app.layer_dialog_custom_color_open);

        // 3. First Escape closes custom color drawer; second Escape closes the dialog
        run_frame(&ctx, &mut app, vec![egui::Key::Escape]);
        assert!(app.show_layer_dialog);
        assert!(!app.layer_dialog_custom_color_open);

        run_frame(&ctx, &mut app, vec![egui::Key::Escape]);
        assert!(!app.show_layer_dialog);
    }
}

