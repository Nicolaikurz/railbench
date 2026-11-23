use super::*;

pub fn draw_left_sidebar(ctx: &egui::Context, app: &mut RailbenchApp) {
    let p = palette(app.dark_mode);

    egui::SidePanel::left("left_sidebar")
        .exact_width(276.0)
        .resizable(false)
        .frame(
            Frame::new()
                .fill(p.bg_sidebar)
                .inner_margin(Margin::same(10))
                .stroke(Stroke::new(1.0_f32, p.border_main)),
        )
        .show(ctx, |ui| {
            let panel_w = 256.0_f32;
            ui.set_min_width(panel_w);
            ui.set_max_width(panel_w);
            match app.sidebar_tab {
                SidebarTab::Tracks => draw_catalog_tab(ui, app, false),
                SidebarTab::Roads => draw_catalog_tab(ui, app, true),
                SidebarTab::Models => draw_models_tab(ui, app),
                SidebarTab::Terrain => draw_terrain_tab(ui, app),
                SidebarTab::Board => draw_board_tab(ui, app),
                SidebarTab::Bom => draw_bom_tab(ui, app),
            }
        });
}

pub(super) fn draw_catalog_tab(ui: &mut egui::Ui, app: &mut RailbenchApp, roads_only: bool) {
    let t = tr(app.lang);
    let p = palette(app.dark_mode);

    let indices: Vec<usize> = app
        .catalogs
        .iter()
        .enumerate()
        .filter(|(_, c)| (c.is_road == Some(true)) == roads_only)
        .map(|(i, _)| i)
        .collect();

    if indices.is_empty() {
        ui.label("No catalogs available.");
        return;
    }

    let sel_idx = if roads_only {
        &mut app.selected_road_catalog_idx
    } else {
        &mut app.selected_catalog_idx
    };
    if !indices.contains(sel_idx) {
        *sel_idx = indices[0];
    }

    let mut trigger_import_catalog = false;
    let imp_tip = if app.lang == Lang::De {
        "Eigenen Gleiskatalog (.json) importieren"
    } else {
        "Import custom track catalog (.json)"
    };

    if roads_only {
        ui.horizontal(|ui| {
            let road_lbl = if app.lang == Lang::De { "STRASSENSYSTEM" } else { "ROAD SYSTEM" };
            ui.label(RichText::new(road_lbl).strong().size(10.0).color(p.text_muted));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(RichText::new("+ JSON").size(10.0))
                    .on_hover_text(imp_tip)
                    .clicked()
                {
                    trigger_import_catalog = true;
                }
            });
        });
        let current_name = app.catalogs[*sel_idx].name.clone();
        egui::ComboBox::from_id_salt("road_cat_sel")
            .width(ui.available_width())
            .selected_text(current_name)
            .show_ui(ui, |ui| {
                for &idx in &indices {
                    let c = &app.catalogs[idx];
                    let lbl = format!("{} ({})", c.name, c.scale);
                    ui.selectable_value(sel_idx, idx, lbl);
                }
            });
    } else {
        // 2-Step Manufacturer -> Track System Selector (matches TS sidebar)
        let mut manufacturers: Vec<String> = Vec::new();
        for &idx in &indices {
            let m = app.catalogs[idx]
                .manufacturer
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Other".into());
            if !manufacturers.contains(&m) {
                manufacturers.push(m);
            }
        }
        manufacturers.sort();

        let mut selected_mfr = app.catalogs[*sel_idx]
            .manufacturer
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| manufacturers[0].clone());

        let mfr_label = if app.lang == Lang::De { "HERSTELLER" } else { "MANUFACTURER" };
        let sys_label = if app.lang == Lang::De { "GLEISSYSTEM" } else { "TRACK SYSTEM" };

        ui.horizontal(|ui| {
            ui.label(RichText::new(mfr_label).strong().size(10.0).color(p.text_muted));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(RichText::new("+ JSON").size(10.0))
                    .on_hover_text(imp_tip)
                    .clicked()
                {
                    trigger_import_catalog = true;
                }
            });
        });
        let prev_mfr = selected_mfr.clone();
        egui::ComboBox::from_id_salt("track_mfr_sel")
            .width(ui.available_width())
            .selected_text(&selected_mfr)
            .show_ui(ui, |ui| {
                for m in &manufacturers {
                    ui.selectable_value(&mut selected_mfr, m.clone(), m);
                }
            });

        // If manufacturer changed, switch to the first catalog of that manufacturer
        if selected_mfr != prev_mfr {
            if let Some(&first_idx) = indices.iter().find(|&&idx| {
                app.catalogs[idx]
                    .manufacturer
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Other")
                    == selected_mfr
            }) {
                *sel_idx = first_idx;
            }
        }

        let sys_indices: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|&idx| {
                app.catalogs[idx]
                    .manufacturer
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Other")
                    == selected_mfr
            })
            .collect();

        ui.add_space(4.0);
        ui.label(RichText::new(sys_label).strong().size(10.0).color(p.text_muted));
        let cur_sys_label = {
            let c = &app.catalogs[*sel_idx];
            let sys = c.system.as_deref().unwrap_or(&c.name);
            format!("{sys} ({})", c.scale)
        };
        egui::ComboBox::from_id_salt("track_sys_sel")
            .width(ui.available_width())
            .selected_text(cur_sys_label)
            .show_ui(ui, |ui| {
                for &idx in &sys_indices {
                    let c = &app.catalogs[idx];
                    let sys = c.system.as_deref().unwrap_or(&c.name);
                    let lbl = format!("{sys} ({})", c.scale);
                    ui.selectable_value(sel_idx, idx, lbl);
                }
            });
    }

    ui.add_space(4.0);
    let search_w = ui.available_width();
    ui.add(
        egui::TextEdit::singleline(&mut app.catalog_search)
            .hint_text(t.search_hint)
            .desired_width(search_w),
    );
    let cat_idx = *sel_idx;
    if trigger_import_catalog {
        app.import_catalog_dialog();
    }

    ui.add_space(4.0);
    let is_de = app.lang == Lang::De;
    let pills: &[(&str, &str)] = if roads_only {
        &[
            ("fav", "★"),
            ("all", if is_de { "Alle" } else { "All" }),
            ("straights", if is_de { "Geraden" } else { "Straights" }),
            ("curves", if is_de { "Kurven" } else { "Curves" }),
            ("crossings", if is_de { "Kreuz." } else { "Crossings" }),
        ]
    } else {
        &[
            ("fav", "★"),
            ("all", if is_de { "Alle" } else { "All" }),
            ("straights", if is_de { "Geraden" } else { "Straights" }),
            ("curves", if is_de { "Kurven" } else { "Curves" }),
            ("turnouts", if is_de { "Weichen" } else { "Turnouts" }),
            ("crossings", if is_de { "Kreuz." } else { "Cross." }),
            ("accessories", if is_de { "Zubehör" } else { "Special" }),
        ]
    };
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(3.5, 3.5);
        for &(cid, clbl) in pills {
            let active = app.catalog_category_filter == cid;
            let txt = RichText::new(clbl).size(10.5).color(if active {
                Color32::WHITE
            } else if cid == "fav" {
                Color32::from_rgb(250, 204, 21)
            } else {
                p.text_sub
            });
            let btn = egui::Button::new(txt)
                .fill(if active { p.primary } else { p.bg_input })
                .corner_radius(CornerRadius::same(6));
            if ui.add(btn).clicked() {
                app.catalog_category_filter = cid.to_string();
            }
        }
    });
    ui.add_space(6.0);

    let filter = app.catalog_search.trim().to_lowercase();
    let cat_filter = app.catalog_category_filter.clone();
    let lang = app.lang;
    let mut dragged_entry_idx: Option<usize> = None;
    let mut clicked_entry_idx: Option<usize> = None;
    let mut toggled_fav_id: Option<String> = None;
    let mut toggled_feeder_id: Option<String> = None;
    let mut toggled_iso_id: Option<String> = None;

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let mut last_cat: &str = "";
        for (entry_idx, entry) in app.catalogs[cat_idx].entries.iter().enumerate() {
            let is_fav = app.app_settings.favorite_tracks.iter().any(|f| f == &entry.id);
            let is_buf = crate::catalog::is_buffer_stop_entry(entry);
            let kind_matches_pill = match cat_filter.as_str() {
                "fav" => is_fav,
                "straights" => matches!(entry.kind.as_str(), "straight" | "flex") && !is_buf,
                "curves" => entry.kind == "curve",
                "turnouts" => matches!(
                    entry.kind.as_str(),
                    "turnout-left"
                        | "turnout-right"
                        | "curved-turnout-left"
                        | "curved-turnout-right"
                        | "three-way"
                        | "double-slip"
                ),
                "crossings" => matches!(entry.kind.as_str(), "crossing" | "double-slip"),
                "accessories" => {
                    is_buf
                        || crate::catalog::is_feeder_entry(entry)
                        || crate::catalog::is_isolating_entry(entry)
                        || matches!(
                            entry.kind.as_str(),
                            "buffer" | "turntable" | "transfer-table" | "feeder" | "uncoupler" | "isolating" | "isolator"
                        )
                }
                _ => true,
            };
            if !kind_matches_pill {
                continue;
            }
            let display_name = if lang == Lang::En {
                entry.name_en.as_deref().unwrap_or(&entry.name)
            } else {
                &entry.name
            };
            if !filter.is_empty()
                && !contains_ignore_case(&entry.id, &filter)
                && !contains_ignore_case(display_name, &filter)
                && !contains_ignore_case(&entry.category, &filter)
            {
                continue;
            }
            if entry.category.as_str() != last_cat {
                if !last_cat.is_empty() {
                    ui.add_space(6.0);
                }
                ui.label(
                    RichText::new(entry.category.to_uppercase())
                        .strong()
                        .size(10.5)
                        .color(p.text_muted),
                );
                last_cat = entry.category.as_str();
            }

            let icon_kind = if is_buf { "buffer" } else { entry.kind.as_str() };
            let accent = match icon_kind {
                "curve" => Color32::from_rgb(59, 130, 246),
                "turnout-left" | "turnout-right" | "three-way" | "double-slip" | "curved-turnout-left" | "curved-turnout-right" => {
                    Color32::from_rgb(245, 158, 11)
                }
                "flex" => Color32::from_rgb(6, 182, 212),
                "crossing" => Color32::from_rgb(168, 85, 247),
                "buffer" => Color32::from_rgb(239, 68, 68),
                "turntable" | "transfer-table" => Color32::from_rgb(14, 165, 233),
                "feeder" => Color32::from_rgb(234, 179, 8),
                _ => Color32::from_rgb(100, 116, 139),
            };

            let dim_text = if let (Some(r), Some(a)) = (entry.radius, entry.angle) {
                format!("R{r:.0} mm • {a:.1}°")
            } else if let Some(l) = entry.length {
                format!("{l:.1} mm")
            } else {
                String::new()
            };

            let is_feeder = crate::catalog::is_feeder_entry(entry);
            let is_iso = crate::catalog::is_isolating_entry(entry);
            let power_enabled = is_feeder && app.catalog_entry_power_enabled.contains(&entry.id);
            let iso_enabled = is_iso && app.catalog_entry_isolating_enabled.contains(&entry.id);

            let mut star_clicked = false;
            let mut feeder_toggle_clicked = false;
            let mut iso_toggle_clicked = false;
            let mut plus_clicked = false;
            let card_w = ui.available_width();
            let card_resp = Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(9, 6))
                .stroke(Stroke::new(1.0_f32, p.border_main))
                .show(ui, |ui| {
                    let inner_w = (card_w - 20.0).max(120.0);
                    ui.set_width(inner_w);
                    ui.set_max_width(inner_w);
                    ui.horizontal(|ui| {
                        let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                        draw_track_kind_icon(ui.painter(), icon_rect, icon_kind, accent, p.bg_input);
                        let col_w = (ui.available_width() - 2.0).max(70.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(col_w, 28.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_max_width(col_w);
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.horizontal(|ui| {
                                    ui.set_max_width(col_w);
                                    ui.label(RichText::new(&entry.id).strong().size(12.5).color(p.text_main));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.spacing_mut().item_spacing.x = 3.0;
                                        if ui
                                            .small_button(RichText::new("+").strong().size(11.0).color(p.primary))
                                            .on_hover_text(if is_de { "Gleis platzieren" } else { "Place track" })
                                            .clicked()
                                        {
                                            plus_clicked = true;
                                        }
                                        let star_sym = if is_fav { "★" } else { "☆" };
                                        let star_col = if is_fav {
                                            Color32::from_rgb(250, 204, 21)
                                        } else {
                                            p.text_muted
                                        };
                                        if ui
                                            .small_button(RichText::new(star_sym).size(11.0).color(star_col))
                                            .on_hover_text(if is_de { "Favorit umschalten" } else { "Toggle favorite" })
                                            .clicked()
                                        {
                                            star_clicked = true;
                                        }
                                        if is_iso {
                                            let iso_tip = if iso_enabled {
                                                if is_de {
                                                    "Trennstrecke aktiv (Klicken zum Ausschalten)"
                                                } else {
                                                    "Track isolator active (click to disable)"
                                                }
                                            } else if is_de {
                                                "Trennstrecke isolieren (Klicken zum Einschalten)"
                                            } else {
                                                "Isolate track (click to enable)"
                                            };
                                            let btn = egui::Button::new(
                                                RichText::new("⊘")
                                                    .strong()
                                                    .size(11.5)
                                                    .color(if iso_enabled { Color32::WHITE } else { p.text_muted }),
                                            )
                                            .fill(if iso_enabled {
                                                Color32::from_rgb(220, 38, 38)
                                            } else {
                                                p.bg_input
                                            })
                                            .small();
                                            if ui.add(btn).on_hover_text(iso_tip).clicked() {
                                                iso_toggle_clicked = true;
                                            }
                                        }
                                        if is_feeder {
                                            let feed_tip = if power_enabled {
                                                if is_de {
                                                    "Gleis-Einspeisung aktiv (Klicken zum Ausschalten)"
                                                } else {
                                                    "Track power feed active (click to disable)"
                                                }
                                            } else if is_de {
                                                "Gleis-Einspeisung hinzufügen (Klicken zum Einschalten)"
                                            } else {
                                                "Add power feed to track (click to enable)"
                                            };
                                            let btn = egui::Button::new(
                                                RichText::new("⚡")
                                                    .strong()
                                                    .size(11.0)
                                                    .color(if power_enabled { Color32::WHITE } else { p.text_muted }),
                                            )
                                            .fill(if power_enabled {
                                                Color32::from_rgb(217, 119, 6)
                                            } else {
                                                p.bg_input
                                            })
                                            .small();
                                            if ui.add(btn).on_hover_text(feed_tip).clicked() {
                                                feeder_toggle_clicked = true;
                                            }
                                        }
                                        if !dim_text.is_empty() {
                                            ui.add(
                                                egui::Label::new(RichText::new(&dim_text).size(10.5).color(p.text_muted))
                                                    .truncate(),
                                            );
                                        }
                                    });
                                });
                                ui.add(
                                    egui::Label::new(RichText::new(display_name).size(11.0).color(p.text_sub))
                                        .truncate(),
                                );
                            },
                        );
                    });
                })
                .response
                .interact(egui::Sense::click_and_drag());

            if card_resp.hovered() {
                ui.painter().rect_stroke(
                    card_resp.rect,
                    CornerRadius::same(8),
                    Stroke::new(1.2_f32, p.primary),
                    egui::StrokeKind::Inside,
                );
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
            if star_clicked {
                toggled_fav_id = Some(entry.id.clone());
            } else if feeder_toggle_clicked {
                toggled_feeder_id = Some(entry.id.clone());
            } else if iso_toggle_clicked {
                toggled_iso_id = Some(entry.id.clone());
            } else if plus_clicked || card_resp.clicked() {
                clicked_entry_idx = Some(entry_idx);
            } else if card_resp.drag_started_by(egui::PointerButton::Primary) {
                dragged_entry_idx = Some(entry_idx);
            }
        }
    });

    if let Some(fid) = toggled_fav_id {
        app.toggle_favorite_track(&fid);
    }
    if let Some(eid) = toggled_feeder_id {
        if !app.catalog_entry_power_enabled.remove(&eid) {
            app.catalog_entry_power_enabled.insert(eid);
        }
    }
    if let Some(eid) = toggled_iso_id {
        if !app.catalog_entry_isolating_enabled.remove(&eid) {
            app.catalog_entry_isolating_enabled.insert(eid);
        }
    }
    if let Some(ei) = dragged_entry_idx {
        app.dragging_catalog_entry = Some(app.catalogs[cat_idx].entries[ei].clone());
    }
    if let Some(ei) = clicked_entry_idx {
        let entry_clone = app.catalogs[cat_idx].entries[ei].clone();
        app.place_catalog_entry(&entry_clone);
    }
}

#[inline]
pub(super) fn contains_ignore_case(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle_lower.is_ascii() {
        let nb = needle_lower.as_bytes();
        return haystack
            .as_bytes()
            .windows(nb.len())
            .any(|w| w.eq_ignore_ascii_case(nb));
    }
    haystack.to_lowercase().contains(needle_lower)
}

pub(super) fn draw_models_tab(ui: &mut egui::Ui, app: &mut RailbenchApp) {
    let p = palette(app.dark_mode);
    let is_de = app.lang == Lang::De;
    ui.label(RichText::new("3D-MODELLE / CUSTOM & BUILTIN").strong().size(11.0).color(p.text_muted));
    ui.add_space(4.0);

    // 1. Import .glb / .gltf card + scale selector
    Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(8))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if ui
                .add_sized(
                    [ui.available_width(), 26.0],
                    egui::Button::new(
                        RichText::new(if is_de { "Import .glb / .gltf" } else { "Import .glb / .gltf" })
                            .strong()
                            .size(11.5)
                            .color(Color32::WHITE),
                    )
                    .fill(p.primary_hover)
                    .corner_radius(CornerRadius::same(6)),
                )
                .clicked()
            {
                app.import_custom_model_dialog();
            }
            ui.add_space(3.0);
            ui.label(
                RichText::new(if is_de {
                    "Oder .glb/.gltf Dateien per Drag & Drop ins Fenster ziehen"
                } else {
                    "Or drag & drop .glb / .gltf files onto window"
                })
                .size(10.0)
                .color(p.text_muted),
            );
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(if is_de { "Massstab:" } else { "Scale:" }).size(11.0).color(p.text_sub));
                let cur_label = model_tools::AVAILABLE_SCALES
                    .iter()
                    .find(|(c, _, _, _)| *c == app.model_scale.as_str())
                    .map(|(_, _, de, en)| if is_de { *de } else { *en })
                    .unwrap_or("Spur H0 (1:87)");
                egui::ComboBox::from_id_salt("models_scale_combo")
                    .width(ui.available_width())
                    .selected_text(cur_label)
                    .show_ui(ui, |ui| {
                        for (code, _, de_lbl, en_lbl) in model_tools::AVAILABLE_SCALES {
                            let lbl = if is_de { *de_lbl } else { *en_lbl };
                            if ui.selectable_label(app.model_scale == *code, lbl).clicked() {
                                app.model_scale = (*code).to_string();
                            }
                        }
                    });
            });
        });

    ui.add_space(6.0);
    ui.add(
        egui::TextEdit::singleline(&mut app.models_search)
            .desired_width(ui.available_width())
            .hint_text(if is_de { "3D-Modelle suchen..." } else { "Search 3D models..." }),
    );
    ui.add_space(4.0);

    ui.horizontal_wrapped(|ui| {
        for (cat, de_lbl, en_lbl) in [
            ("all", "Alle", "All"),
            ("buildings", "Gebaeude", "Buildings"),
            ("railway", "Bahn", "Railway"),
            ("nature", "Natur", "Nature"),
            ("custom", "Eigene", "Custom"),
        ] {
            let lbl = if is_de { de_lbl } else { en_lbl };
            if ui.selectable_label(app.models_category == cat, lbl).clicked() {
                app.models_category = cat.into();
            }
        }
    });
    ui.add_space(4.0);

    let q = app.models_search.trim().to_ascii_lowercase();
    let cat = app.models_category.as_str();
    let f_scale = model_tools::scale_factor_from_h0(&app.model_scale);

    let mut place_model: Option<railbench_core::types::Model3D> = None;
    let mut remove_custom_name: Option<String> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        if cat != "custom" {
            for def in model_tools::BUILTIN_MODELS {
                if cat != "all" && def.category != cat {
                    continue;
                }
                let name = if is_de { def.name_de } else { def.name_en };
                let desc = if is_de { def.desc_de } else { def.desc_en };
                if !q.is_empty()
                    && !name.to_ascii_lowercase().contains(&q)
                    && !desc.to_ascii_lowercase().contains(&q)
                    && !def.id.to_ascii_lowercase().contains(&q)
                {
                    continue;
                }
                let w = (def.width * f_scale * 10.0).round() / 10.0;
                let d = (def.depth * f_scale * 10.0).round() / 10.0;
                let h = (def.height * f_scale * 10.0).round() / 10.0;
                Frame::new()
                    .fill(p.bg_card)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(10, 7))
                    .stroke(Stroke::new(1.0_f32, p.border_main))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(name).strong().size(12.0).color(p.text_main));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("+ Add").clicked() {
                                    place_model = Some(model_tools::create_scaled_builtin_model(
                                        def,
                                        &app.model_scale,
                                        is_de,
                                    ));
                                }
                            });
                        });
                        ui.label(
                            RichText::new(format!("{w:.0} x {d:.0} x {h:.0} mm ({})", app.model_scale))
                                .size(10.5)
                                .color(p.primary),
                        );
                        ui.label(RichText::new(desc).size(10.0).color(p.text_muted));
                    });
                ui.add_space(4.0);
            }
        }

        if cat == "all" || cat == "custom" {
            for cm in &app.custom_models_library {
                if !q.is_empty() && !cm.name.to_ascii_lowercase().contains(&q) {
                    continue;
                }
                Frame::new()
                    .fill(p.bg_card)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(10, 7))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(99, 102, 241)))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&cm.name).strong().size(12.0).color(p.text_main));
                            ui.label(RichText::new(format!(".{}", cm.model_type)).size(10.0).color(Color32::from_rgb(129, 140, 248)));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("Del").clicked() {
                                    remove_custom_name = Some(cm.name.clone());
                                }
                                if ui.small_button("+ Add").clicked() {
                                    place_model = Some(cm.clone());
                                }
                            });
                        });
                        ui.label(
                            RichText::new(format!("{:.0} x {:.0} x {:.0} mm", cm.width, cm.depth, cm.height))
                                .size(10.5)
                                .color(p.text_muted),
                        );
                    });
                ui.add_space(4.0);
            }
        }
    });

    if let Some(m) = place_model {
        app.place_model_instance(m);
    }
    if let Some(rm) = remove_custom_name {
        app.custom_models_library.retain(|m| m.name != rm);
    }
}

