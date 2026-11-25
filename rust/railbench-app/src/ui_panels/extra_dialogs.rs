use super::*;

pub(super) fn draw_welcome_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let mut open = app.show_welcome_dialog;
    let mut action: Option<u8> = None;

    let recent_path = app
        .app_settings
        .last_opened_path
        .clone()
        .filter(|path| std::path::Path::new(path).exists());

    egui::Window::new(if is_de {
        "Willkommen bei RAILbench"
    } else {
        "Welcome to RAILbench"
    })
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    .default_width(480.0)
    .show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("RAILbench 2D & 3D  {}", crate::version::APP_VERSION_TAG))
                    .strong()
                    .size(20.0)
                    .color(p.primary),
            );
            ui.label(
                RichText::new(if is_de {
                    "Professioneller Modelleisenbahn-Gleisplaner (Native Desktop)"
                } else {
                    "Professional Model Railway Layout Planner (Native Desktop)"
                })
                .size(12.0)
                .color(p.text_muted),
            );
        });
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        let btn_w = ui.available_width();
        if let Some(ref recent) = recent_path {
            let fname = std::path::Path::new(recent)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(recent);
            let lbl = if is_de {
                format!("Letzten Plan fortsetzen ({fname})")
            } else {
                format!("Continue Last Plan ({fname})")
            };
            if ui
                .add_sized([btn_w, 36.0], egui::Button::new(RichText::new(lbl).strong()))
                .clicked()
            {
                action = Some(1);
            }
            ui.add_space(4.0);
        }

        if ui
            .add_sized(
                [btn_w, 34.0],
                egui::Button::new(if is_de {
                    "Neue leere Anlage erstellen"
                } else {
                    "Create New Empty Layout"
                }),
            )
            .clicked()
        {
            action = Some(2);
        }
        ui.add_space(4.0);

        if ui
            .add_sized(
                [btn_w, 34.0],
                egui::Button::new(if is_de {
                    "Anlage aus Datei oeffnen..."
                } else {
                    "Open Layout From File..."
                }),
            )
            .clicked()
        {
            action = Some(3);
        }
        ui.add_space(4.0);

        if ui
            .add_sized(
                [btn_w, 34.0],
                egui::Button::new(if is_de {
                    "Demo-Anlagen entdecken (Spur H0, 0, N, G, Z)..."
                } else {
                    "Explore Demo Layouts (Scale H0, 0, N, G, Z)..."
                }),
            )
            .clicked()
        {
            action = Some(4);
        }

        ui.add_space(10.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut app.welcome_remember_choice,
                if is_de {
                    "Auswahl fuer naechsten Programmstart merken"
                } else {
                    "Remember choice for next startup"
                },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(if is_de { "Schliessen" } else { "Close" }).clicked() {
                    action = Some(0);
                }
            });
        });
    });

    app.show_welcome_dialog = open;
    if let Some(act) = action {
        match act {
            1 => {
                if let Some(ref recent) = recent_path {
                    app.open_layout_path(recent);
                }
                if app.welcome_remember_choice {
                    app.app_settings.startup_action = StartupAction::Recent;
                    app.save_current_settings();
                }
                app.show_welcome_dialog = false;
            }
            2 => {
                app.new_empty_layout();
                if app.welcome_remember_choice {
                    app.app_settings.startup_action = StartupAction::New;
                    app.save_current_settings();
                }
                app.show_welcome_dialog = false;
            }
            3 => {
                app.open_layout_dialog();
                app.show_welcome_dialog = false;
            }
            4 => {
                if app.welcome_remember_choice {
                    app.app_settings.startup_action = StartupAction::Demo;
                    app.save_current_settings();
                }
                app.show_welcome_dialog = false;
                app.show_demo_dialog = true;
            }
            _ => {
                app.show_welcome_dialog = false;
            }
        }
    }
}

pub(super) fn draw_demo_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let mut open = app.show_demo_dialog;
    let mut chosen_path: Option<&'static str> = None;

    const DEMOS: &[(&str, &str, &str, &str, &str)] = &[
        (
            "H0",
            "Alpenland H0 Demo",
            "demotracks/alpenland-h0.railbench",
            "Gebirgsanlage mit Tunneln, Bruecken, Bahnhof und Landschaft",
            "Mountain layout with tunnels, bridges, station & terrain",
        ),
        (
            "H0",
            "Mainline H0",
            "demotracks/mainline-h0.railbench",
            "Zweigleisiges Hauptbahn-Oval mit Ausweichgleisen & Stromkreisen",
            "Double-track mainline oval with sidings & power feeders",
        ),
        (
            "0",
            "Shunting Yard 0",
            "demotracks/shunting-0.railbench",
            "Rangierbahnhof im Massstab 1:45 (Lenz Spur 0)",
            "Industrial switching yard in 1:45 (Spur 0)",
        ),
        (
            "N",
            "Switching N",
            "demotracks/switching-n.railbench",
            "Kompakte Spur-N Timesaver-Rangieranlage",
            "Compact Scale N timesaver shunting layout",
        ),
        (
            "G",
            "Garden G",
            "demotracks/garden-g.railbench",
            "Gartenbahn mit R1-Boegen & Abstellgleisen (LGB Spur G)",
            "Outdoor garden railway with R1 curves & spurs (LGB G)",
        ),
        (
            "Z",
            "Suitcase Z",
            "demotracks/suitcase-z.railbench",
            "Kompakte Kofferanlage in 1:220 (Maerklin mini-club Spur Z)",
            "Compact suitcase layout in 1:220 (Scale Z)",
        ),
    ];

    egui::Window::new(if is_de {
        "Demo-Anlagen"
    } else {
        "Demo Layouts"
    })
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(520.0)
    .show(ctx, |ui| {
        ui.label(
            RichText::new(if is_de {
                "Waehlen Sie eine vorkonfigurierte Beispielanlage zum Laden:"
            } else {
                "Choose a ready-to-run sample layout to load:"
            })
            .size(12.0)
            .color(p.text_muted),
        );
        ui.add_space(6.0);

        for (scale, title, path, desc_de, desc_en) in DEMOS {
            let demo_meta = app.demo_preview_meta(path);
            let available = std::path::Path::new(path).exists() || crate::RailbenchApp::embedded_demo_json(path).is_some();
            Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(7))
                .inner_margin(Margin::same(9))
                .stroke(Stroke::new(1.0_f32, p.border_main))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        Frame::new()
                            .fill(p.primary_hover)
                            .corner_radius(CornerRadius::same(5))
                            .inner_margin(Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.label(RichText::new(*scale).strong().size(12.0).color(Color32::WHITE));
                            });
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(RichText::new(*title).strong().size(13.0).color(p.text_main));
                            ui.label(
                                RichText::new(if is_de { *desc_de } else { *desc_en })
                                    .size(11.0)
                                    .color(p.text_muted),
                            );
                            if let Some(ref meta) = demo_meta {
                                ui.label(
                                    RichText::new(meta)
                                        .size(10.0)
                                        .color(p.primary),
                                );
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    available,
                                    egui::Button::new(if is_de { "Laden" } else { "Load" }),
                                )
                                .clicked()
                            {
                                chosen_path = Some(*path);
                            }
                        });
                    });
                });
            ui.add_space(4.0);
        }
    });

    app.show_demo_dialog = open;
    if let Some(path) = chosen_path {
        app.request_open_demo(path);
        app.show_demo_dialog = false;
        app.show_welcome_dialog = false;
    }
}

pub(super) fn draw_settings_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let mut open = app.show_settings_dialog;
    let mut dirty_settings = false;
    let mut reset_personal = false;

    egui::Window::new(if is_de {
        "Einstellungen & Tastenkuerzel"
    } else {
        "Settings & Keyboard Shortcuts"
    })
    .open(&mut open)
    .collapsible(false)
    .resizable(true)
    .default_size([600.0, 480.0])
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut app.settings_tab,
                0,
                if is_de { "Allgemein" } else { "General" },
            );
            ui.selectable_value(
                &mut app.settings_tab,
                1,
                if is_de { "Tastenkuerzel" } else { "Shortcuts" },
            );
        });
        ui.separator();

        if app.settings_tab == 0 {
            app.recording_key_action = None;
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(RichText::new(if is_de { "Oberflaeche & Startverhalten" } else { "Appearance & Startup" }).strong());
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Sprache:" } else { "Language:" });
                    let is_auto = app.app_settings.lang == "auto";
                    if ui.radio(!is_auto && app.lang == Lang::De, "Deutsch").clicked() {
                        app.app_settings.lang = "de".into();
                        app.lang = Lang::De;
                        dirty_settings = true;
                    }
                    if ui.radio(!is_auto && app.lang == Lang::En, "English").clicked() {
                        app.app_settings.lang = "en".into();
                        app.lang = Lang::En;
                        dirty_settings = true;
                    }
                    if ui.radio(is_auto, if is_de { "System (Auto)" } else { "System (Auto)" }).clicked() {
                        app.app_settings.lang = "auto".into();
                        app.lang = if crate::settings::detect_os_lang_de() { Lang::De } else { Lang::En };
                        dirty_settings = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Design:" } else { "Theme:" });
                    dirty_settings |= ui.radio_value(&mut app.dark_mode, true, if is_de { "Dunkel" } else { "Dark" }).changed();
                    dirty_settings |= ui.radio_value(&mut app.dark_mode, false, if is_de { "Hell" } else { "Light" }).changed();
                });
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Beim Programmstart:" } else { "On Startup:" });
                    let cur = app.app_settings.startup_action;
                    egui::ComboBox::from_id_salt("settings_startup_action")
                        .width(240.0)
                        .selected_text(cur.label(is_de))
                        .show_ui(ui, |ui| {
                            for act in [
                                StartupAction::Ask,
                                StartupAction::Recent,
                                StartupAction::New,
                                StartupAction::Demo,
                                StartupAction::Open,
                            ] {
                                if ui.selectable_value(&mut app.app_settings.startup_action, act, act.label(is_de)).changed() {
                                    dirty_settings = true;
                                }
                            }
                        });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.label(RichText::new(if is_de { "2D-Editor & Werkzeuge" } else { "2D Editor & Tools" }).strong());
                dirty_settings |= ui.checkbox(&mut app.snapping_enabled, if is_de { "Einrasten aktiv (Snapping)" } else { "Enable Snapping" }).changed();
                dirty_settings |= ui.checkbox(&mut app.rotation_snap_locked, if is_de { "Drehschritt sperren (Fixiert)" } else { "Lock Rotation Snap Angle" }).changed();
                dirty_settings |= ui.checkbox(&mut app.power_sim_enabled, if is_de { "Stromanalyse-Overlay standardmaessig aktiv" } else { "Enable Power Overlay by Default" }).changed();
                dirty_settings |= ui.checkbox(&mut app.show_labels, if is_de { "Artikelnummern anzeigen" } else { "Show Track Labels" }).changed();
                dirty_settings |= ui.checkbox(&mut app.show_connectors, if is_de { "Verbinder anzeigen" } else { "Show Connectors" }).changed();
                dirty_settings |= ui.checkbox(&mut app.show_rulers, if is_de { "Lineale & Koordinatengitter" } else { "Rulers & Grid" }).changed();
                dirty_settings |= ui.checkbox(&mut app.show_wood_plates, if is_de { "Holzplatten in 2D/3D rendern" } else { "Render Wood Plates in 2D/3D" }).changed();
                dirty_settings |= ui.checkbox(&mut app.show_right_sidebar, if is_de { "Schwebendes Ebenen-Menue rechts sichtbar" } else { "Floating Layers Menu (Right)" }).changed();
                dirty_settings |= ui.checkbox(&mut app.low_gpu_mode, if is_de { "Energiesparmodus (Low-GPU)" } else { "Low-GPU Power Saving Mode" }).changed();
                dirty_settings |= ui.checkbox(&mut app.perf_monitor.visible, "Hardware Monitor (F3)").changed();

                ui.horizontal(|ui| {
                    ui.label(if is_de { "Drehwinkel-Schritt:" } else { "Rotation Step:" });
                    egui::ComboBox::from_id_salt("settings_rot_step")
                        .width(90.0)
                        .selected_text(format!("{} deg", app.rotation_step))
                        .show_ui(ui, |ui| {
                            for deg in [5.7, 7.5, 15.0, 24.3, 30.0, 45.0, 90.0] {
                                if ui.selectable_value(&mut app.rotation_step, deg, format!("{deg} deg")).changed() {
                                    dirty_settings = true;
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Farbmodus:" } else { "Color Mode:" });
                    dirty_settings |= ui.radio_value(&mut app.color_mode, ColorMode::Types, if is_de { "Typen" } else { "Types" }).changed();
                    dirty_settings |= ui.radio_value(&mut app.color_mode, ColorMode::Layers, if is_de { "Ebenen" } else { "Layers" }).changed();
                    dirty_settings |= ui.radio_value(&mut app.color_mode, ColorMode::Off, if is_de { "Aus" } else { "Off" }).changed();
                });

                ui.add_space(8.0);
                ui.separator();
                ui.label(RichText::new(if is_de { "3D-Umgebung" } else { "3D Viewport" }).strong());
                ui.horizontal(|ui| {
                    dirty_settings |= ui
                        .radio_value(
                            &mut app.viewer3d_env,
                            crate::viewer3d::Viewer3DEnv::HallLandscape,
                            if is_de { "Halle & Landschaft" } else { "Hall & Landscape" },
                        )
                        .changed();
                    dirty_settings |= ui
                        .radio_value(
                            &mut app.viewer3d_env,
                            crate::viewer3d::Viewer3DEnv::Minimal,
                            "CAD / Minimal",
                        )
                        .changed();
                });

                ui.add_space(12.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(if is_de { "Standardwerte wiederherstellen" } else { "Reset to Defaults" }).clicked() {
                        reset_personal = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(p_cfg) = crate::settings::AppSettings::config_path() {
                            ui.weak(format!("{}", p_cfg.display()));
                        }
                    });
                });
            });
        } else {
            // Keymap Editor Tab
            if let Some(rec_action) = app.recording_key_action {
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(14, 165, 233, 45))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(8))
                    .stroke(Stroke::new(1.2_f32, p.primary))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(if is_de {
                                    "Tastenkombination druecken (Esc zum Abbrechen)..."
                                } else {
                                    "Press desired key combination (Esc to cancel)..."
                                })
                                .strong()
                                .color(p.primary),
                            );
                            if ui.small_button(if is_de { "Abbrechen" } else { "Cancel" }).clicked() {
                                app.recording_key_action = None;
                            }
                        });
                    });
                ui.add_space(4.0);
                match ctx.input(capture_pressed_shortcut) {
                    KeyCapture::Cancelled => {
                        app.recording_key_action = None;
                    }
                    KeyCapture::Captured(binding) => {
                        if let Some(conflict) = app.app_settings.find_conflicting_action(rec_action, &binding) {
                            let conflict_lbl = if is_de { conflict.label_de } else { conflict.label_en };
                            app.app_settings.set_key(conflict.action, String::new());
                            app.show_toast(if is_de {
                                format!("Taste '{binding}' neu zugewiesen (von '{conflict_lbl}' entfernt)")
                            } else {
                                format!("Reassigned '{binding}' (cleared from '{conflict_lbl}')")
                            });
                        }
                        app.app_settings.set_key(rec_action, binding);
                        app.recording_key_action = None;
                        dirty_settings = true;
                    }
                    KeyCapture::Pending => {}
                }
            }

            ui.horizontal(|ui| {
                ui.label(if is_de { "Suche:" } else { "Search:" });
                ui.add(
                    egui::TextEdit::singleline(&mut app.keymap_search)
                        .desired_width(180.0)
                        .hint_text(if is_de { "Aktion oder Taste..." } else { "Action or key..." }),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.confirm_reset_keys {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(if is_de { "Wirklich?" } else { "Confirm Reset?" })
                                        .color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(220, 38, 38)),
                            )
                            .clicked()
                        {
                            app.app_settings.reset_all_keys();
                            app.recording_key_action = None;
                            app.confirm_reset_keys = false;
                            dirty_settings = true;
                            app.show_toast(if is_de {
                                "Alle Tastenkuerzel zurueckgesetzt"
                            } else {
                                "All keyboard shortcuts reset"
                            });
                        }
                        if ui.button(if is_de { "Abbrechen" } else { "Cancel" }).clicked() {
                            app.confirm_reset_keys = false;
                        }
                    } else if ui
                        .button(if is_de {
                            "Alle zuruecksetzen"
                        } else {
                            "Reset All Keys"
                        })
                        .clicked()
                    {
                        app.confirm_reset_keys = true;
                    }
                });
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                let cats: &[(Option<KeyCategory>, &str)] = &[
                    (None, if is_de { "Alle" } else { "All" }),
                    (Some(KeyCategory::File), if is_de { "Datei" } else { "File" }),
                    (Some(KeyCategory::Edit), if is_de { "Bearbeiten" } else { "Edit" }),
                    (Some(KeyCategory::View), if is_de { "Ansicht" } else { "View" }),
                    (Some(KeyCategory::Tools), if is_de { "Werkzeuge" } else { "Tools" }),
                ];
                for &(cat_opt, lbl) in cats {
                    let active = app.keymap_category == cat_opt;
                    if ui.selectable_label(active, lbl).clicked() {
                        app.keymap_category = cat_opt;
                    }
                }
            });
            ui.add_space(4.0);

            let q = app.keymap_search.trim().to_ascii_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for def in KEY_ACTION_DEFS {
                    if let Some(cat) = app.keymap_category {
                        if def.category != cat {
                            continue;
                        }
                    }
                    let lbl = if is_de { def.label_de } else { def.label_en };
                    let desc = if is_de { def.desc_de } else { def.desc_en };
                    let cur_key = app.app_settings.effective_key(def.action);
                    if !q.is_empty()
                        && !lbl.to_ascii_lowercase().contains(&q)
                        && !desc.to_ascii_lowercase().contains(&q)
                        && !cur_key.to_ascii_lowercase().contains(&q)
                    {
                        continue;
                    }

                    let is_recording = app.recording_key_action == Some(def.action);
                    let is_custom = cur_key != def.default_key;

                    Frame::new()
                        .fill(p.bg_card)
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(8, 5))
                        .stroke(Stroke::new(1.0_f32, if is_recording { p.primary } else { p.border_main }))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    ui.label(RichText::new(lbl).strong().size(12.0).color(p.text_main));
                                    ui.label(RichText::new(desc).size(10.5).color(p.text_muted));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if is_custom
                                        && ui
                                            .small_button(if is_de { "Std" } else { "Reset" })
                                            .on_hover_text(def.default_key)
                                            .clicked()
                                    {
                                        app.app_settings.reset_key(def.action);
                                        dirty_settings = true;
                                    }
                                    let btn_txt = if is_recording {
                                        if is_de { "[Taste druecken]" } else { "[Press Key]" }.to_string()
                                    } else {
                                        cur_key.clone()
                                    };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(btn_txt)
                                                    .strong()
                                                    .size(11.5)
                                                    .color(if is_recording { Color32::WHITE } else { p.primary }),
                                            )
                                            .fill(if is_recording { p.primary_hover } else { p.bg_sidebar }),
                                        )
                                        .clicked()
                                    {
                                        app.recording_key_action = if is_recording {
                                            None
                                        } else {
                                            Some(def.action)
                                        };
                                    }
                                });
                            });
                        });
                    ui.add_space(3.0);
                }
            });
        }
    });

    app.show_settings_dialog = open;
    if !open {
        app.recording_key_action = None;
    }
    if reset_personal {
        app.reset_personal_settings();
    } else if dirty_settings {
        app.save_current_settings();
    }
}

pub(super) fn draw_help_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let mut open = app.show_help_dialog;
    let mut open_keymap = false;

    egui::Window::new(if is_de {
        "RAILbench Hilfe & Handbuch"
    } else {
        "RAILbench Help & User Guide"
    })
    .open(&mut open)
    .collapsible(false)
    .resizable(true)
    .default_size([640.0, 480.0])
    .show(ctx, |ui| {
        let tabs = if is_de {
            [
                "Ueberblick",
                "Gleise & Flex",
                "Ebenen & Rampen",
                "Strom & Oberleitung",
                "Grundplatten",
                "3D & Landschaft",
                "Tastenkuerzel",
                "Export",
                "Katalog-JSON",
                "Sync & Info",
            ]
        } else {
            [
                "Overview",
                "Tracks & Flex",
                "Layers & Ramps",
                "Power & Catenary",
                "Baseboards",
                "3D & Scenery",
                "Shortcuts",
                "Export",
                "Catalog JSON",
                "Collab & About",
            ]
        };

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            for (idx, lbl) in tabs.iter().enumerate() {
                let active = app.help_tab as usize == idx;
                if ui.selectable_label(active, *lbl).clicked() {
                    app.help_tab = idx as u8;
                }
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| match app.help_tab {
            0 => {
                ui.heading(format!(
                    "RAILbench Native 2D & 3D Planner ({})",
                    crate::version::APP_VERSION_TAG
                ));
                ui.add_space(4.0);
                ui.label(if is_de {
                    "RAILbench verbindet praezise 2D-Gleisplanung (H0, TT, N, Z, 0, 1, G + Strassen) mit direkter 3D-Ansicht, elektrischer Stromkreis-Analyse und professionellem Mehrseiten-Bauplanexport."
                } else {
                    "RAILbench combines precision 2D layout design (H0, TT, N, Z, 0, 1, G + Roads) with live 3D rendering, circuit & short-circuit simulation, and multi-page vector blueprint export."
                });
                ui.add_space(6.0);
                ui.label(if is_de {
                    "Workflow: Gleis oder Strasse links auswaehlen -> per Klick oder Drag & Drop platzieren -> freie Verbinder docken automatisch an -> mit '3' direkt in die 3D-Ansicht wechseln."
                } else {
                    "Workflow: Pick a track/road on the left -> click or drag onto canvas -> open endpoints auto-snap -> press '3' anytime for live 3D inspection."
                });
            }
            1 => {
                ui.heading(if is_de { "Gleise, Strassen & Flexgleise" } else { "Tracks, Roads & Flextracks" });
                ui.add_space(4.0);
                ui.label(if is_de {
                    "- Auto-Snap: Gleise snappen an Gleise, Strassen separat an Strassen.\n- '+' Schnell-Anfuegen: Klick auf den blauen '+'-Button eines offenen Verbinders verlaengert direkt mit dem zuletzt gewaehlten Stueck.\n- Flexgleise: Ausgewaehlte Flexgleise bieten 3 blaue Biegepunkte (25%, 50%, 75%) sowie 2 Endpunkte zum Kuerzen, Biegen und exakten magnetischen Einrasten (Alt = freies Ziehen)."
                } else {
                    "- Auto-Snap: Tracks snap cleanly to tracks, roads snap separately to roads.\n- '+' Quick-Add: Click the cyan '+' badge on any open connector to extend immediately.\n- Flextracks: Selected flex pieces reveal 3 interior spline bend points (25%, 50%, 75%) plus 2 endpoint handles that bend, shorten, and magnetically snap with 0° tangent alignment (hold Alt to bypass snap)."
                });
            }
            2 => {
                ui.heading(if is_de { "Ebenen, Steigungen & Gleiswendeln" } else { "Layers, Ramps & Helices" });
                ui.add_space(4.0);
                ui.label(if is_de {
                    "- Rechts im Ebenen-Panel verwalten Sie Ebenen-Hoehen (mm), Sichtbarkeit, Sperrung und Farben.\n- Auto-Rampe: Waehlen Sie zwei verbundene oder benachbarte Gleise und waehlen Sie im Hoehen-Dialog lineare oder S-Kurven-Uebergaenge mit Gefaelle-Check.\n- Wendel-Generator: Erstellt kreisfoermige Gleiswendeln automatisch passend zum Durchfahrtsmassstab."
                } else {
                    "- Use the right Layers panel to control layer elevations (mm), visibility, locking, and colors.\n- Auto-Ramp: Compute smooth linear or S-curve grade transitions with gradient checks.\n- Helix Generator: Generate multi-turn spiral helices respecting scale clearance heights."
                });
            }
            3 => {
                ui.heading(if is_de { "Stromkreise, Kurzschluss & Oberleitung" } else { "Power Simulation, Shorts & Catenary" });
                ui.add_space(4.0);
                ui.label(if is_de {
                    "- Einspeisungen & Trennstellen: Mit 'F' Stromeinspeisung und 'I' Trennstelle setzen (Shift+F / Shift+I fuer Oberleitung).\n- 2-Leiter-Kehrschleifen werden automatisch auf Kurzschluesse geprueft und direkt in 2D und 3D (rote Bake) markiert.\n- Im Strom-Dialog lassen sich 12V-24V Presets, parallele/getrennte Oberleitung und Weichen-Decoder/Laternen konfigurieren."
                } else {
                    "- Feeders & Isolators: Press 'F' to toggle track feeders, 'I' for rail isolators (Shift+F / Shift+I for catenary).\n- 2-wire reversing loops are checked live for short circuits, highlighted on 2D and marked with a red beacon in 3D.\n- Configure voltage presets, separate/parallel catenary mode, and turnout lanterns/decoders in the Power dialog."
                });
            }
            4 => {
                ui.heading(if is_de { "Mehr-Tisch Grundplatten & CAD Sketcher" } else { "Multi-Table Baseboards & CAD Sketcher" });
                ui.add_space(4.0);
                ui.label(if is_de {
                    "- Mehrere Tische kombinieren: Rechteck, L-Form, U-Form, T-Form, Dreieck oder freie Polygone.\n- Im Board-Tab lassen sich Tische verschieben (Kanten-Snapping) und per CAD Sketcher (Polygon, Rechteck, Ecken-Editor) zeichnen.\n- Pro Ebene koennen Trassenbretter ('Holzplatten') inklusive automatischer Gleisausschnitte aktiviert werden."
                } else {
                    "- Combine multiple baseboard tables: Rectangle, L-shape, U-shape, T-shape, Triangle, or Custom polygons.\n- Use the Board tab and interactive CAD Sketcher (Polygon, Rect, Vertex Edit) with ortho & grid snap.\n- Enable per-layer wood sub-plates with automatic track corridor cutouts."
                });
            }
            5 => {
                ui.heading(if is_de { "3D-Ansicht, Modelle & Landschaft" } else { "3D Viewport, Models & Terrain" });
                ui.add_space(4.0);
                ui.label(if is_de {
                    "- 16 integrierte 3D-Gebaeude & Baeume mit automatischer Massstabsanpassung sowie Lokschuppen-Torsnapping.\n- Eigene .glb / .gltf 3D-Modelle per Datei-Dialog oder Drag & Drop importieren.\n- Berg-Pinsel & Boden-Platten modellieren direkt 3D-Gelaende.\n- 360-Grad Orbit-Kamera, CAD-ViewCube, Halle & Landschaft / Minimal-Umgebungen sowie Tag/Nacht-Beleuchtung in beiden Modi."
                } else {
                    "- 16 scale-aware builtin 3D structures, plus roundhouse/shed door snapping for tracks.\n- Import custom .glb / .gltf models via File Dialog or Drag & Drop.\n- Paint mountains interactively with the Mountain Brush.\n- Full 360/180 orbit camera, CAD ViewCube, Hall & Landscape / Minimal scenes, and Day/Night lighting in both modes."
                });
            }
            6 => {
                ui.horizontal(|ui| {
                    ui.heading(if is_de { "Aktive Tastenkuerzel" } else { "Active Keyboard Shortcuts" });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(if is_de { "Im Editor anpassen..." } else { "Customize Shortcuts..." })
                            .clicked()
                        {
                            open_keymap = true;
                        }
                    });
                });
                ui.add_space(4.0);
                for def in KEY_ACTION_DEFS {
                    let lbl = if is_de { def.label_de } else { def.label_en };
                    let key = app.app_settings.effective_key(def.action);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(key).strong().color(p.primary));
                        ui.label(format!("-- {lbl}"));
                    });
                }
            }
            7 => {
                ui.heading(if is_de { "Export & Druck" } else { "Export & Blueprints" });
                ui.add_space(4.0);
                ui.label(if is_de {
                    "- .railbench (JSON): Vollstaendiges Projektformat inkl. Ebenen, Stromparametern und eingebetteten 3D-Modellen.\n- Stueckliste CSV: Semikolon-getrennt (Excel DE-kompatibel) nach Gleisen, Strassen und Modellen mit Meter-Summen.\n- XML (.xml), 1:1 mm SVG (mit Raster & Elektrik-Symbolen) und Mehrseiten-PDF (Uebersicht, Ebenen-Blaetter mit Ghosting, Stueckliste)."
                } else {
                    "- .railbench (JSON): Full lossless layout format with layers, electrical config, and embedded models.\n- BOM CSV: Semicolon-delimited summary separating tracks, roads, and 3D buildings with length totals.\n- XML (.xml), 1:1 mm SVG (with optional grid & electrical annotations), and multi-page Vector PDF (Overview, Per-Layer sheets with ghosting, BOM page)."
                });
            }
            8 => {
                ui.heading(if is_de { "Eigenen Gleiskatalog (JSON-Schema)" } else { "Custom Track Catalog JSON Schema" });
                ui.add_space(4.0);
                let sample_json = r#"{
  "id": "custom-h0",
  "name": "Mein Katalog H0",
  "scale": "H0",
  "gauge": 16.5,
  "voltage": 16,
  "hasRoadbed": true,
  "tracks": [
    { "id": "C100", "name": "Gerade 200mm", "type": "straight", "length": 200 },
    { "id": "C200", "name": "Bogen R2 30°", "type": "curve", "radius": 420, "angle": 30 },
    { "id": "C300", "name": "Weiche Links", "type": "turnout-left", "length": 200, "radius": 420, "branchAngle": 24 },
    { "id": "C400", "name": "Flexgleis 900mm", "type": "flex", "length": 900 }
  ]
}"#;
                ui.label(if is_de {
                    "Unterstuetzte Import-Formate: Standard-Objekt { id, name, scale, gauge, tracks[] }, direktes Gleis-Array [...] sowie deutsche Feldnamen (laenge, gleise, spur, spurweite)."
                } else {
                    "Supported import formats: Standard catalog object { id, name, scale, gauge, tracks[] }, bare track array [...], and German field aliases (laenge, gleise, spur, spurweite)."
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(if is_de { "Beispiel-JSON kopieren" } else { "Copy Example JSON" }).clicked() {
                        ui.ctx().copy_text(sample_json.to_string());
                        app.show_toast(if is_de {
                            "Katalog-Beispiel in Zwischenablage kopiert"
                        } else {
                            "Example catalog JSON copied to clipboard"
                        });
                    }
                    if ui.button(if is_de { "Beispiel-JSON speichern..." } else { "Export Example JSON..." }).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("custom-catalog-example.json")
                            .add_filter("Catalog JSON", &["json"])
                            .save_file()
                        {
                            if std::fs::write(&path, sample_json).is_ok() {
                                app.show_toast(if is_de {
                                    "Beispiel-Katalog gespeichert"
                                } else {
                                    "Example catalog saved"
                                });
                            }
                        }
                    }
                });
                ui.add_space(6.0);
                egui::Frame::new()
                    .fill(p.bg_sidebar)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.monospace(sample_json);
                    });
            }
            _ => {
                ui.heading(if is_de { "Live-Sync Server, CLI & Ueber RAILbench" } else { "Collab Server, CLI Flags & About" });
                ui.add_space(4.0);
                ui.label(if is_de {
                    "WebSocket Live-Collaboration:\n- Eingebetteter Server (Port 3000, automatischer Fallback bis +20) mit Raum-Speicherung unter ~/.railbench/saved_plans/<raum>.railbench.\n- Zeigt farbige Live-Cursor und Auswahl-Markierungen aller Teilnehmer sowie integrierten Chat."
                } else {
                    "WebSocket Live Collaboration:\n- Built-in host server (port 3000 with auto port fallback up to +20) and automatic room persistence in ~/.railbench/saved_plans/<room>.railbench.\n- Live colored cursors, selection highlights per user, profile updates, LAN invite URLs, and real-time chat."
                });
                ui.add_space(6.0);
                ui.monospace(
                    "railbench-app --server --port 3000 --room club --password secret\nrailbench-app --connect ws://192.168.1.20:3000/collab --room club\nENV: RAILBENCH_SERVER=1  RAILBENCH_PORT=3000  RAILBENCH_ROOM=...  RAILBENCH_PASSWORD=...",
                );
                ui.add_space(8.0);
                ui.separator();
                ui.label(format!(
                    "RAILbench Native Desktop ({}) — Rust / egui / wgpu",
                    crate::version::APP_VERSION_TAG
                ));
            }
        });
    });

    app.show_help_dialog = open;
    if open_keymap {
        app.show_help_dialog = false;
        app.settings_tab = 1;
        app.show_settings_dialog = true;
    }
}

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
    egui::Window::new(if is_de {
        "Gleiseigenschaften & Information"
    } else {
        "Track Properties & Info"
    })
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(380.0)
    .show(ctx, |ui| {
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

    egui::Window::new(if is_de {
        "Gleis teilen oder Typ ersetzen"
    } else {
        "Split Track or Replace Type"
    })
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(440.0)
    .show(ctx, |ui| {
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

    egui::Window::new(if is_edit {
        if is_de { "Ebene bearbeiten" } else { "Edit Layer" }
    } else if is_de {
        "Neue Ebene erstellen"
    } else {
        "Create New Layer"
    })
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(360.0)
    .show(ctx, |ui| {
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
    egui::Window::new(if is_de { "Ebene loeschen?" } else { "Delete Layer?" })
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .show(ctx, |ui| {
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

    egui::Window::new(if is_de {
        "Ungesicherte Aenderungen?"
    } else {
        "Unsaved Changes?"
    })
    .collapsible(false)
    .resizable(false)
    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    .default_width(370.0)
    .show(ctx, |ui| {
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

    egui::Window::new(if is_de {
        "Automatische Sicherung gefunden"
    } else {
        "Unsaved Session Recovery"
    })
    .collapsible(false)
    .resizable(false)
    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    .default_width(380.0)
    .show(ctx, |ui| {
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

    match act {
        Some(true) => app.restore_autosave(),
        Some(false) => app.discard_autosave(),
        None => {}
    }
}

