use super::*;

pub(super) fn draw_settings_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let mut open = app.show_settings_dialog;
    let mut dirty_settings = false;
    let mut reset_personal = false;

    let settings_title = if is_de {
        "Einstellungen & Tastenkuerzel"
    } else {
        "Settings & Keyboard Shortcuts"
    };
    egui::Window::new(settings_title)
        .id(egui::Id::new("railbench_settings_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_size([600.0, 480.0])
        .frame(crate::theme::railbench_window_frame(app.dark_mode))
        .show(ctx, |ui| {
            crate::theme::show_railbench_dialog_body(ui, app.dark_mode, settings_title, Some(&mut open), 12, |ui| {
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
                            dirty_settings = true;
                        }
                    }
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
                dirty_settings |= ui.checkbox(&mut app.bars_locked, if is_de { "Leisten / Seitenleisten sperren" } else { "Lock Sidebar & Panel Sizes" }).changed();
                dirty_settings |= ui.checkbox(&mut app.low_gpu_mode, if is_de { "Energiesparmodus (Low-GPU)" } else { "Low-GPU Power Saving Mode" }).changed();
                dirty_settings |= ui.checkbox(&mut app.use_discrete_gpu, if is_de { "Dedizierte GPU fuer 3D bevorzugen (Neustart)" } else { "Prefer Discrete GPU for 3D (Requires Restart)" }).changed();
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

    let help_title = if is_de {
        "RAILbench Hilfe & Handbuch"
    } else {
        "RAILbench Help & User Guide"
    };
    egui::Window::new(help_title)
        .id(egui::Id::new("railbench_help_dialog"))
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_size([640.0, 480.0])
        .frame(crate::theme::railbench_window_frame(app.dark_mode))
        .show(ctx, |ui| {
            crate::theme::show_railbench_dialog_body(ui, app.dark_mode, help_title, Some(&mut open), 12, |ui| {
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
    });

    app.show_help_dialog = open;
    if open_keymap {
        app.show_help_dialog = false;
        app.settings_tab = 1;
        app.show_settings_dialog = true;
    }
}
