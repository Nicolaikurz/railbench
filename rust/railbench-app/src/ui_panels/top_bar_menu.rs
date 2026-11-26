use super::*;

fn fixed_menu_button<R>(
    ui: &mut egui::Ui,
    active_lbl: &str,
    de_lbl: &str,
    en_lbl: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) {
    let slot_w = stable_text_width(ui, de_lbl, en_lbl, None, false)
        + ui.spacing().button_padding.x * 2.0
        + 4.0;
    ui.allocate_ui_with_layout(
        egui::vec2(slot_w, ui.spacing().interact_size.y),
        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        |ui| ui.menu_button(active_lbl, add_contents),
    );
}

pub(super) fn draw_menu_bar(ctx: &egui::Context, app: &mut RailbenchApp) {
    let t = tr(app.lang);
    let p = palette(app.dark_mode);
    // =========================================================================
    //  LINE 1: Menubar Row (Brand + Menus on left; Filename + Track count right)
    //  Matches src/ui/toolbar.ts toolbar-menubar-row
    // =========================================================================
    egui::TopBottomPanel::top("menu_bar")
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(p.bg_toolbar)
                .inner_margin(Margin::symmetric(12, 5))
                .stroke(Stroke::NONE),
        )
        .show(ctx, |ui| {
            style_toolbar_ui(ui, p);
            egui::menu::bar(ui, |ui| {
                // RAILbench brand badge + CalVer tag (click opens Welcome dialog)
                let brand_resp = ui
                    .horizontal(|ui| {
                        ui.label(RichText::new("RAIL").strong().size(14.5).color(p.primary));
                        ui.add_space(-6.0);
                        ui.label(RichText::new("bench").strong().size(14.5).color(Color32::WHITE));
                        ui.add_space(-2.0);
                        ui.label(
                            RichText::new(crate::version::APP_VERSION_TAG)
                                .size(10.5)
                                .monospace()
                                .color(Color32::from_rgb(148, 163, 184)),
                        );
                    })
                    .response
                    .interact(egui::Sense::click())
                    .on_hover_text(if app.lang == Lang::De {
                        "Willkommensbildschirm & Aktuelles oeffnen"
                    } else {
                        "Open Welcome & Quick Start"
                    });
                if brand_resp.clicked() {
                    app.show_welcome_dialog = true;
                }
                ui.add_space(10.0);

                fixed_menu_button(ui, t.menu_file, DE.menu_file, EN.menu_file, |ui| {
                    if ui.button(t.new_layout).clicked() {
                        app.request_new_layout();
                        ui.close_menu();
                    }
                    if ui.button(t.open_file).clicked() {
                        app.request_open_layout_dialog();
                        ui.close_menu();
                    }
                    let recent_lbl = if app.lang == Lang::De {
                        "Zuletzt geoeffnet"
                    } else {
                        "Open Recent"
                    };
                    let recents_existing: Vec<String> = app
                        .app_settings
                        .recent_files
                        .iter()
                        .filter(|p| std::path::Path::new(p).exists())
                        .cloned()
                        .collect();
                    ui.menu_button(recent_lbl, |ui| {
                        if recents_existing.is_empty() {
                            ui.weak(if app.lang == Lang::De {
                                "(Keine aktuellen Dateien)"
                            } else {
                                "(No recent files)"
                            });
                        } else {
                            for path in &recents_existing {
                                let fname = std::path::Path::new(path)
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(path);
                                if ui.button(fname).on_hover_text(path).clicked() {
                                    app.open_layout_path(path);
                                    ui.close_menu();
                                }
                            }
                            ui.separator();
                            if ui
                                .button(if app.lang == Lang::De {
                                    "Liste leeren"
                                } else {
                                    "Clear Recent"
                                })
                                .clicked()
                            {
                                app.app_settings.recent_files.clear();
                                app.save_current_settings();
                                ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button(t.open_demo, |ui| {
                        for (label, path) in [
                            ("Alpenland H0 Demo", "demotracks/alpenland-h0.railbench"),
                            ("Mainline H0", "demotracks/mainline-h0.railbench"),
                            ("Shunting 0", "demotracks/shunting-0.railbench"),
                            ("Switching N", "demotracks/switching-n.railbench"),
                            ("Garden G", "demotracks/garden-g.railbench"),
                            ("Suitcase Z", "demotracks/suitcase-z.railbench"),
                        ] {
                            if ui.button(label).clicked() {
                                app.request_open_demo(path);
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        let all_demos_lbl = if app.lang == Lang::De {
                            "Alle Demo-Anlagen..."
                        } else {
                            "Demo Gallery..."
                        };
                        if ui.button(all_demos_lbl).clicked() {
                            app.show_demo_dialog = true;
                            ui.close_menu();
                        }
                    });
                    let save_lbl = if app.lang == Lang::De { "Speichern (Strg+S)" } else { "Save (Ctrl+S)" };
                    if ui.button(save_lbl).clicked() {
                        app.save_layout_dialog();
                        ui.close_menu();
                    }
                    let save_as_lbl = if app.lang == Lang::De { "Speichern unter... (Strg+Shift+S)" } else { "Save As... (Ctrl+Shift+S)" };
                    if ui.button(save_as_lbl).clicked() {
                        app.save_layout_as_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    let print_lbl = if app.lang == Lang::De {
                        "Bauplan-Export & Drucken..."
                    } else {
                        "Blueprint Export & Print..."
                    };
                    if ui.button(print_lbl).clicked() {
                        app.show_export_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button("Export PDF Blueprint...").clicked() {
                        app.export_pdf_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Export SVG Blueprint...").clicked() {
                        app.export_svg_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Export XML (.xml)...").clicked() {
                        app.export_xml_dialog();
                        ui.close_menu();
                    }
                    let export_parts_lbl = if app.lang == Lang::De {
                        "Teileliste exportieren (CSV)..."
                    } else {
                        "Export Part List (CSV)..."
                    };
                    if ui.button(export_parts_lbl).clicked() {
                        app.export_bom_csv_dialog();
                        ui.close_menu();
                    }
                    let import_cat_lbl = if app.lang == Lang::De {
                        "Gleiskatalog importieren (.json)..."
                    } else {
                        "Import Track Catalog (.json)..."
                    };
                    if ui.button(import_cat_lbl).clicked() {
                        app.import_catalog_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Collaborate (WebSocket)...").clicked() {
                        app.show_collab_dialog = true;
                        ui.close_menu();
                    }
                    let welcome_lbl = if app.lang == Lang::De {
                        "Startbildschirm..."
                    } else {
                        "Welcome Screen..."
                    };
                    if ui.button(welcome_lbl).clicked() {
                        app.show_welcome_dialog = true;
                        ui.close_menu();
                    }
                });

                fixed_menu_button(ui, t.menu_edit, DE.menu_edit, EN.menu_edit, |ui| {
                    let undo_menu_lbl = if app.lang == Lang::De {
                        "Rückgängig (Strg+Z)"
                    } else {
                        "Undo (Ctrl+Z)"
                    };
                    let redo_menu_lbl = if app.lang == Lang::De {
                        "Wiederholen (Strg+Y)"
                    } else {
                        "Redo (Ctrl+Y)"
                    };
                    if ui.add_enabled(!app.undo_stack.is_empty(), egui::Button::new(undo_menu_lbl)).clicked() {
                        app.undo();
                        ui.close_menu();
                    }
                    if ui.add_enabled(!app.redo_stack.is_empty(), egui::Button::new(redo_menu_lbl)).clicked() {
                        app.redo();
                        ui.close_menu();
                    }
                    ui.separator();
                    let copy_lbl = if app.lang == Lang::De { "Kopieren (Strg+C)" } else { "Copy (Ctrl+C)" };
                    let paste_lbl = if app.lang == Lang::De { "Einfügen (Strg+V)" } else { "Paste (Ctrl+V)" };
                    let dup_lbl = if app.lang == Lang::De { "Duplizieren (Strg+D)" } else { "Duplicate (Ctrl+D)" };
                    let sel_all_lbl = if app.lang == Lang::De { "Alles auswählen (Strg+A)" } else { "Select All (Ctrl+A)" };
                    let sel_conn_lbl = if app.lang == Lang::De { "Verbundene Gleise auswählen" } else { "Select Connected Tracks" };
                    if ui.add_enabled(!app.selected.is_empty(), egui::Button::new(copy_lbl)).clicked() {
                        app.copy_selected();
                        ui.close_menu();
                    }
                    if ui.add_enabled(app.clipboard.is_some(), egui::Button::new(paste_lbl)).clicked() {
                        app.paste_clipboard(None);
                        ui.close_menu();
                    }
                    if ui.add_enabled(!app.selected.is_empty(), egui::Button::new(dup_lbl)).clicked() {
                        app.duplicate_selected();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(sel_all_lbl).clicked() {
                        app.select_all();
                        ui.close_menu();
                    }
                    if ui.add_enabled(!app.selected.is_empty(), egui::Button::new(sel_conn_lbl)).clicked() {
                        app.select_connected_tracks();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(t.rotate_ccw).clicked() {
                        app.rotate_selected(-app.rotation_step);
                        ui.close_menu();
                    }
                    if ui.button(t.rotate_cw).clicked() {
                        app.rotate_selected(app.rotation_step);
                        ui.close_menu();
                    }
                    let is_de_edit = app.lang == Lang::De;
                    if ui
                        .add_enabled(
                            !app.selected.is_empty(),
                            egui::Button::new(if is_de_edit {
                                "Spiegeln / Wenden (Alt+F)"
                            } else {
                                "Flip Selected (Alt+F)"
                            }),
                        )
                        .clicked()
                    {
                        app.flip_selected();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            !app.selected.is_empty(),
                            egui::Button::new(if is_de_edit {
                                "Gruppieren / Entgruppieren (G)"
                            } else {
                                "Group / Ungroup (G)"
                            }),
                        )
                        .clicked()
                    {
                        app.toggle_group_selected();
                        ui.close_menu();
                    }
                    let one_piece_id = if app.selected.len() == 1 {
                        app.layout
                            .pieces
                            .iter()
                            .find(|p| app.selected.contains(&p.id))
                            .map(|p| p.id.clone())
                    } else {
                        None
                    };
                    if ui
                        .add_enabled(
                            one_piece_id.is_some(),
                            egui::Button::new(if is_de_edit {
                                "Gleis teilen / ersetzen... (Strg+Shift+T)"
                            } else {
                                "Split / Replace Track... (Ctrl+Shift+T)"
                            }),
                        )
                        .clicked()
                    {
                        app.replace_track_search.clear();
                        app.show_replace_track_piece_id = one_piece_id.clone();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            one_piece_id.is_some(),
                            egui::Button::new(if is_de_edit {
                                "Gleis-Eigenschaften... (Alt+Enter)"
                            } else {
                                "Track Properties... (Alt+Enter)"
                            }),
                        )
                        .clicked()
                    {
                        app.show_track_props_piece_id = one_piece_id;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button(if is_de_edit {
                            "Hoehe & Rampe... (M)"
                        } else {
                            "Height & Ramp... (M)"
                        })
                        .clicked()
                    {
                        app.show_height_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            !app.selected.is_empty(),
                            egui::Button::new(if is_de_edit {
                                "Auto-Rampe anwenden (Shift+M)"
                            } else {
                                "Apply Auto-Ramp (Shift+M)"
                            }),
                        )
                        .clicked()
                    {
                        app.apply_auto_ramp_one_shot();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(t.delete_sel).clicked() {
                        app.delete_selected();
                        ui.close_menu();
                    }
                });

                fixed_menu_button(ui, t.menu_view, DE.menu_view, EN.menu_view, |ui| {
                    let is_de = app.lang == Lang::De;
                    if ui.button(t.reset_view).clicked() {
                        app.pending_fit_view = true;
                        ui.close_menu();
                    }
                    let view3d_lbl = if is_de { "3D-Ansicht umschalten (3)" } else { "Toggle 3D View (3)" };
                    if ui.button(view3d_lbl).clicked() {
                        app.toggle_3d();
                        ui.close_menu();
                    }
                    ui.separator();
                    let mut view_changed = false;
                    view_changed |= ui.checkbox(&mut app.show_labels, format!("{} (N)", t.labels)).changed();
                    view_changed |= ui.checkbox(&mut app.show_connectors, t.connectors).changed();
                    view_changed |= ui
                        .checkbox(&mut app.show_rulers, if is_de { "Lineale & Raster (Shift+L)" } else { "Rulers & Grid (Shift+L)" })
                        .changed();
                    view_changed |= ui.checkbox(&mut app.snapping_enabled, format!("{} (S)", t.snapping)).changed();
                    view_changed |= ui
                        .checkbox(
                            &mut app.layer_protection,
                            if is_de {
                                "Ebenenschutz (Alt+L)"
                            } else {
                                "Layer Protection (Alt+L)"
                            },
                        )
                        .changed();
                    view_changed |= ui
                        .checkbox(&mut app.show_wood_plates, if is_de { "Holzplatten anzeigen" } else { "Show Wood Plates" })
                        .changed();
                    view_changed |= ui.checkbox(&mut app.show_right_sidebar, format!("{} (Shift+B)", t.layers_title)).changed();
                    ui.checkbox(&mut app.power_sim_enabled, format!("{} (P)", t.volt_btn));
                    view_changed |= ui.checkbox(&mut app.perf_monitor.visible, "Hardware Monitor (F3)").changed();
                    view_changed |= ui
                        .checkbox(
                            &mut app.bars_locked,
                            if is_de { "Seitenleisten sperren" } else { "Lock Sidebars & Panels" },
                        )
                        .changed();
                    let is_fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    if ui
                        .checkbox(&mut is_fs.clone(), if is_de { "Vollbild (F11)" } else { "Fullscreen (F11)" })
                        .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fs));
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.menu_button(if is_de { "Design / Theme" } else { "Theme" }, |ui| {
                        for tm in [
                            crate::settings::ThemeMode::System,
                            crate::settings::ThemeMode::Dark,
                            crate::settings::ThemeMode::Light,
                        ] {
                            let active = app.app_settings.theme_mode == tm;
                            if ui.radio(active, tm.label(is_de)).clicked() {
                                app.app_settings.theme_mode = tm;
                                match tm {
                                    crate::settings::ThemeMode::Dark => app.dark_mode = true,
                                    crate::settings::ThemeMode::Light => app.dark_mode = false,
                                    crate::settings::ThemeMode::System => {
                                        if let Some(st) = ctx.system_theme() {
                                            app.dark_mode = matches!(st, egui::Theme::Dark);
                                        }
                                    }
                                }
                                app.save_current_settings();
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    view_changed |= ui.radio_value(&mut app.color_mode, ColorMode::Types, t.color_types).changed();
                    view_changed |= ui.radio_value(&mut app.color_mode, ColorMode::Layers, t.color_layers).changed();
                    view_changed |= ui.radio_value(&mut app.color_mode, ColorMode::Off, t.color_off).changed();
                    if view_changed {
                        app.save_current_settings();
                    }
                });

                fixed_menu_button(ui, t.menu_settings, DE.menu_settings, EN.menu_settings, |ui| {
                    let is_de = app.lang == Lang::De;
                    if ui.button(t.power_dialog).clicked() {
                        app.show_power_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button(t.height_dialog).clicked() {
                        app.show_height_dialog = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button(if is_de {
                            "Einstellungen & Tastenkuerzel..."
                        } else {
                            "Settings & Shortcuts..."
                        })
                        .clicked()
                    {
                        app.settings_tab = 0;
                        app.show_settings_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Tastaturkuerzel anpassen..."
                        } else {
                            "Customize Shortcuts..."
                        })
                        .clicked()
                    {
                        app.settings_tab = 1;
                        app.show_settings_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Bauplan-Druck & Export..."
                        } else {
                            "Blueprint Print & Export..."
                        })
                        .clicked()
                    {
                        app.show_export_dialog = true;
                        ui.close_menu();
                    }
                });

                fixed_menu_button(ui, t.menu_help, DE.menu_help, EN.menu_help, |ui| {
                    let is_de = app.lang == Lang::De;
                    if ui
                        .button(if is_de {
                            "Hilfe & Handbuch..."
                        } else {
                            "Help & User Guide..."
                        })
                        .clicked()
                    {
                        app.help_tab = 0;
                        app.show_help_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Tastenkuerzel"
                        } else {
                            "Keyboard Shortcuts"
                        })
                        .clicked()
                    {
                        app.help_tab = 6;
                        app.show_help_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Eigenen Katalog (JSON-Schema)..."
                        } else {
                            "Catalog JSON Schema..."
                        })
                        .clicked()
                    {
                        app.help_tab = 8;
                        app.show_help_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Bauplan-Druck & Export-Anleitung..."
                        } else {
                            "Blueprint Print & Export Guide..."
                        })
                        .clicked()
                    {
                        app.help_tab = 7;
                        app.show_help_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Collab Server & CLI-Optionen..."
                        } else {
                            "Collab Server & CLI Flags..."
                        })
                        .clicked()
                    {
                        app.help_tab = 9;
                        app.show_help_dialog = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button(if is_de {
                            "Demo-Anlagen..."
                        } else {
                            "Demo Layouts..."
                        })
                        .clicked()
                    {
                        app.show_demo_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Willkommensbildschirm..."
                        } else {
                            "Welcome Screen..."
                        })
                        .clicked()
                    {
                        app.show_welcome_dialog = true;
                        ui.close_menu();
                    }
                });

                // Right side of Line 1: Window Controls (Close / Max / Min) + Track Count + Filename
                let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    crate::theme::draw_os_window_controls(ui, ctx, p);

                    ui.add_space(8.0);
                    let cnt = app.layout.pieces.len();
                    let badge_w = stable_text_width(
                        ui,
                        &format!("{cnt} {}", DE.status_pieces),
                        &format!("{cnt} {}", EN.status_pieces),
                        Some(11.5),
                        true,
                    );
                    Frame::new()
                        .fill(p.toolbar_btn)
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(badge_w, 16.0),
                                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!("{cnt} {}", t.status_pieces))
                                            .size(11.5)
                                            .strong()
                                            .color(p.primary),
                                    );
                                },
                            );
                        });
                    ui.add_space(6.0);
                    let dirty = app.is_dirty();
                    let fname_lbl = if dirty {
                        format!("• {}.railbench", app.layout.name)
                    } else {
                        format!("{}.railbench", app.layout.name)
                    };
                    let fname_col = if dirty {
                        Color32::from_rgb(251, 191, 36)
                    } else {
                        Color32::from_rgb(203, 213, 225)
                    };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(fname_lbl).size(12.0).color(fname_col))
                                .frame(false),
                        )
                        .on_hover_text(if app.lang == Lang::De {
                            "Klicken zum Speichern (Strg+S)"
                        } else {
                            "Click to save layout (Ctrl+S)"
                        })
                        .clicked()
                    {
                        app.save_layout_dialog();
                    }

                    // Fill remaining space in Row 1 as a draggable titlebar (drag to move, double-click to maximize)
                    let drag_rect = ui.available_rect_before_wrap();
                    if drag_rect.width() > 8.0 {
                        let drag_resp = ui.interact(
                            drag_rect,
                            egui::Id::new("custom_titlebar_drag"),
                            egui::Sense::click_and_drag(),
                        );
                        if drag_resp.double_clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        } else if drag_resp.drag_started_by(egui::PointerButton::Primary) {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                    }
                });
            });
        });

    // Subtle 2 px lighter divider strip separating menubar and actions toolbar
    egui::TopBottomPanel::top("main_toolbar_divider")
        .exact_height(2.0)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(p.toolbar_btn_hover)
                .inner_margin(Margin::ZERO)
                .stroke(Stroke::NONE),
        )
        .show(ctx, |_| {});

}
