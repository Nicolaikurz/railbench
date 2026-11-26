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
    let layer_name = layer.map(|l| l.name.as_str()).unwrap_or("Ebene 0");
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
                    .map(|e| if !is_de { e.name_en.as_deref().unwrap_or(&e.name) } else { e.name.as_str() })
                    .unwrap_or("-");
                row(if is_de { "Bezeichnung:" } else { "Name / Type:" }, name.to_string());
                row(
                    if is_de { "Kategorie:" } else { "Category:" },
                    entry.map(|e| e.kind.as_str()).unwrap_or("-").to_string(),
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
                        format!(
                            "{}, {}",
                            piece.signal_side.as_deref().unwrap_or("right"),
                            piece.signal_direction.as_deref().unwrap_or("forward")
                        )
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
                        || cand.name_en.as_deref().unwrap_or("").to_ascii_lowercase().contains(&q));
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

pub(super) fn draw_layer_edit_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let is_edit = app.layer_dialog_edit_id.is_some();
    let mut open = app.show_layer_dialog;
    let mut submit = false;

    const LAYER_COLORS: &[&str] = &[
        "#38bdf8", "#34d399", "#fbbf24", "#f472b6", "#a78bfa",
        "#f97316", "#2dd4bf", "#ef4444", "#60a5fa", "#94a3b8",
    ];

    let layer_title = if is_edit {
        if is_de { "Ebene bearbeiten" } else { "Edit Layer" }
    } else if is_de {
        "Neue Ebene erstellen"
    } else {
        "Create New Layer"
    };
    egui::Window::new(layer_title)
        .id(egui::Id::new("railbench_layer_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .default_width(360.0)
        .frame(crate::theme::railbench_window_frame(app.dark_mode))
        .show(ctx, |ui| {
            crate::theme::show_railbench_dialog_body(ui, app.dark_mode, layer_title, Some(&mut open), 12, |ui| {
        ui.horizontal(|ui| {
            ui.label(if is_de { "Name:" } else { "Name:" });
            ui.text_edit_singleline(&mut app.layer_dialog_name);
        });
        ui.horizontal(|ui| {
            ui.label(if is_de { "Hoehe:" } else { "Elevation:" });
            ui.add(egui::DragValue::new(&mut app.layer_dialog_height_mm).speed(5.0).suffix(" mm"));
        });
        ui.add_space(4.0);
        ui.label(if is_de { "Farbe:" } else { "Color:" });
        ui.horizontal_wrapped(|ui| {
            for &hex in LAYER_COLORS {
                let col = crate::canvas2d::parse_hex_color(Some(hex)).unwrap_or(p.primary);
                let active = app.layer_dialog_color.eq_ignore_ascii_case(hex);
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 4.0, col);
                if active {
                    ui.painter().rect_stroke(rect, 4.0, Stroke::new(2.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
                }
                if resp.clicked() {
                    app.layer_dialog_color = hex.into();
                }
            }
        });
        ui.add_space(6.0);
        ui.label(if is_de { "Trassenbrett / Holzplatte:" } else { "Sub-Plate / Roadbed Deck:" });
        ui.horizontal(|ui| {
            for (mode, lbl) in [
                ("none", if is_de { "Keine" } else { "None" }),
                ("table", if is_de { "Vollplatte" } else { "Table Deck" }),
                ("tight", if is_de { "Trassenbrett" } else { "Track Contour" }),
            ] {
                if ui.selectable_label(app.layer_dialog_wood_mode == mode, lbl).clicked() {
                    app.layer_dialog_wood_mode = mode.into();
                }
            }
        });
        if app.layer_dialog_wood_mode != "none" {
            ui.horizontal(|ui| {
                ui.label(if is_de { "Plattenstaerke:" } else { "Thickness:" });
                ui.add(
                    egui::DragValue::new(&mut app.layer_dialog_wood_thickness)
                        .range(2.0..=50.0)
                        .speed(1.0)
                        .suffix(" mm"),
                );
            });
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(if is_de { "Abbrechen" } else { "Cancel" }).clicked() {
                app.show_layer_dialog = false;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new(if is_edit {
                        if is_de { "Speichern" } else { "Save" }
                    } else if is_de {
                        "Erstellen"
                    } else {
                        "Create"
                    }).strong())
                    .clicked()
                {
                    submit = true;
                }
            });
        });
            });
    });

    if submit {
        app.apply_layer_dialog();
    } else if !open {
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

