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
            ui.label(RichText::new("RAILbench 2D & 3D").strong().size(20.0).color(p.primary));
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
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let exists = std::path::Path::new(path).exists();
                            if ui
                                .add_enabled(
                                    exists,
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
        app.open_layout_path(path);
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
                    dirty_settings |= ui.radio_value(&mut app.lang, Lang::De, "Deutsch").changed();
                    dirty_settings |= ui.radio_value(&mut app.lang, Lang::En, "English").changed();
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
                            for act in [StartupAction::Ask, StartupAction::Recent, StartupAction::New, StartupAction::Demo] {
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
                dirty_settings |= ui.checkbox(&mut app.show_labels, if is_de { "Artikelnummern anzeigen" } else { "Show Track Labels" }).changed();
                dirty_settings |= ui.checkbox(&mut app.show_connectors, if is_de { "Verbinder anzeigen" } else { "Show Connectors" }).changed();
                dirty_settings |= ui.checkbox(&mut app.show_rulers, if is_de { "Lineale & Koordinatengitter" } else { "Rulers & Grid" }).changed();
                dirty_settings |= ui.checkbox(&mut app.show_wood_plates, if is_de { "Holzplatten in 2D/3D rendern" } else { "Render Wood Plates in 2D/3D" }).changed();
                dirty_settings |= ui.checkbox(&mut app.show_right_sidebar, if is_de { "Schwebendes Ebenen-Menue rechts sichtbar" } else { "Floating Layers Menu (Right)" }).changed();
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
                    if ui
                        .button(if is_de {
                            "Alle zuruecksetzen"
                        } else {
                            "Reset All Keys"
                        })
                        .clicked()
                    {
                        app.app_settings.reset_all_keys();
                        app.recording_key_action = None;
                        dirty_settings = true;
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
                "Export & Sync",
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
                "Export & Sync",
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
                ui.heading("RAILbench Native 2D & 3D Planner");
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
            _ => {
                ui.heading(if is_de { "Export, Druck & Live-Kollaboration" } else { "Export, Blueprints & Live Sync" });
                ui.add_space(4.0);
                ui.label(if is_de {
                    "- .railbench (JSON): Vollstaendiges Projektformat inkl. Ebenen, Stromparametern und eingebetteten 3D-Modellen.\n- Stueckliste CSV: Semikolon-getrennt (Excel DE-kompatibel) nach Gleisen, Strassen und Modellen mit Meter-Summen.\n- XML (.xml) & 1:1 mm SVG sowie Mehrseiten-PDF (Uebersicht, Ebenen-Blaetter mit Ghosting, Stueckliste).\n- WebSocket Live-Collaboration fuer gleichzeitiges Planen im Team."
                } else {
                    "- .railbench (JSON): Full lossless layout format with layers, electrical config, and embedded models.\n- BOM CSV: Semicolon-delimited summary separating tracks, roads, and 3D buildings with length totals.\n- XML (.xml), 1:1 mm SVG, and multi-page Vector PDF (Overview, Per-Layer sheets with ghosting, BOM page).\n- WebSocket Live Collaboration for multi-user planning."
                });
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

