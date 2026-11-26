use super::*;
use eframe::egui::{FontId, Pos2, Rect, Sense};

pub(super) fn draw_catalog_tab(ui: &mut egui::Ui, app: &mut RailbenchApp, roads_only: bool) {
    let t = tr(app.lang);
    let p = palette(app.dark_mode);
    let is_de = app.lang == Lang::De;

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

    let cur_sel_ref = if roads_only {
        &mut app.selected_road_catalog_idx
    } else {
        &mut app.selected_catalog_idx
    };
    if !indices.contains(cur_sel_ref) {
        *cur_sel_ref = indices[0];
    }
    let prev_cat_idx = *cur_sel_ref;
    let mut cur_sel_idx = *cur_sel_ref;

    if roads_only
        && matches!(
            app.catalog_picker_step,
            crate::CatalogPickerStep::Manufacturers | crate::CatalogPickerStep::Systems(_)
        )
    {
        app.catalog_picker_step = crate::CatalogPickerStep::Closed;
    } else if !roads_only && matches!(app.catalog_picker_step, crate::CatalogPickerStep::RoadScales) {
        app.catalog_picker_step = crate::CatalogPickerStep::Closed;
    }

    let is_open = app.catalog_picker_step != crate::CatalogPickerStep::Closed;
    let show_favorites = !roads_only && app.catalog_category_filter == "fav";
    let fav_count = app.app_settings.favorite_tracks.len();

    let (trigger_title, trigger_sub) = {
        let cat = &app.catalogs[cur_sel_idx];
        (
            format_catalog_trigger_title(cat, roads_only, show_favorites, is_de),
            format_catalog_trigger_subtitle(cat, roads_only, show_favorites, fav_count, is_de),
        )
    };

    let trigger_w = ui.available_width();
    let (trigger_rect, trigger_resp) = ui.allocate_exact_size(egui::vec2(trigger_w, 44.0), Sense::click());
    {
        let painter = ui.painter_at(trigger_rect);
        let fill = if trigger_resp.hovered() || is_open {
            p.bg_card_hover
        } else {
            p.bg_card
        };
        let stroke = Stroke::new(
            if is_open || trigger_resp.hovered() { 1.2_f32 } else { 1.0_f32 },
            if is_open || trigger_resp.hovered() {
                p.primary
            } else {
                p.border_main
            },
        );
        painter.rect(trigger_rect, 8.0, fill, stroke, egui::StrokeKind::Inside);
        crate::icons::paint_chevron(
            &painter,
            Pos2::new(trigger_rect.max.x - 14.0, trigger_rect.center().y),
            if is_open {
                crate::icons::ChevronDir::Up
            } else {
                crate::icons::ChevronDir::Down
            },
            3.8,
            p.text_muted,
        );
        let text_clip = Rect::from_min_max(
            Pos2::new(trigger_rect.min.x + 11.0, trigger_rect.min.y),
            Pos2::new(trigger_rect.max.x - 26.0, trigger_rect.max.y),
        );
        let tp = ui.painter_at(text_clip);
        let title_col = if show_favorites {
            favorite_accent(app.dark_mode, Color32::from_rgb(245, 158, 11))
        } else {
            p.text_main
        };
        tp.text(
            Pos2::new(text_clip.min.x, trigger_rect.min.y + 8.0),
            egui::Align2::LEFT_TOP,
            &trigger_title,
            FontId::proportional(12.5),
            title_col,
        );
        tp.text(
            Pos2::new(text_clip.min.x, trigger_rect.min.y + 25.0),
            egui::Align2::LEFT_TOP,
            &trigger_sub,
            FontId::proportional(10.5),
            p.text_muted,
        );
    }

    let mut trigger_import_catalog = false;
    let mut trigger_export_catalog = false;
    let mut next_step: Option<crate::CatalogPickerStep> = None;
    let mut next_cat_filter: Option<String> = None;

    if trigger_resp.clicked() {
        next_step = Some(if is_open {
            crate::CatalogPickerStep::Closed
        } else if roads_only {
            crate::CatalogPickerStep::RoadScales
        } else {
            crate::CatalogPickerStep::Manufacturers
        });
    }

    if is_open {
        let popover_id = ui.make_persistent_id(if roads_only {
            "road_catalog_popover"
        } else {
            "track_catalog_popover"
        });
        let popover_w = trigger_rect.width().max(240.0);
        let popover_bg = if app.dark_mode {
            Color32::from_rgba_unmultiplied(24, 34, 52, 248)
        } else {
            Color32::from_rgba_unmultiplied(250, 246, 238, 252)
        };
        let popover_border = if app.dark_mode {
            Color32::from_rgba_unmultiplied(148, 163, 184, 72)
        } else {
            Color32::from_rgba_unmultiplied(15, 23, 42, 42)
        };
        let header_bg = if app.dark_mode {
            Color32::from_rgba_unmultiplied(0, 0, 0, 42)
        } else {
            Color32::from_rgba_unmultiplied(15, 23, 42, 12)
        };
        let header_border = if app.dark_mode {
            Color32::from_rgba_unmultiplied(255, 255, 255, 26)
        } else {
            Color32::from_rgba_unmultiplied(15, 23, 42, 22)
        };

        let area_resp = egui::Area::new(popover_id)
            .fixed_pos(Pos2::new(trigger_rect.min.x, trigger_rect.max.y + 4.0))
            .order(egui::Order::Foreground)
            .constrain(true)
            .show(ui.ctx(), |ui| {
                Frame::new()
                    .fill(popover_bg)
                    .stroke(Stroke::new(1.0_f32, popover_border))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(0))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 20,
                        spread: 0,
                        color: Color32::from_black_alpha(if app.dark_mode { 115 } else { 42 }),
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(popover_w);
                        ui.set_max_width(popover_w);
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 1.0);

                        // Popover Header Band
                        let (hdr_rect, _) = ui.allocate_exact_size(egui::vec2(popover_w, 30.0), Sense::hover());
                        let hp = ui.painter_at(hdr_rect);
                        hp.rect_filled(
                            hdr_rect,
                            CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 },
                            header_bg,
                        );
                        hp.hline(hdr_rect.x_range(), hdr_rect.max.y - 0.5, Stroke::new(1.0_f32, header_border));

                        match &app.catalog_picker_step {
                            crate::CatalogPickerStep::Manufacturers => {
                                let hdr_txt = if is_de {
                                    "1. HERSTELLER WÄHLEN"
                                } else {
                                    "1. SELECT MANUFACTURER"
                                };
                                hp.text(
                                    Pos2::new(hdr_rect.min.x + 12.0, hdr_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    hdr_txt,
                                    FontId::proportional(10.5),
                                    p.text_muted,
                                );
                            }
                            crate::CatalogPickerStep::Systems(active_mfg) => {
                                let back_rect = Rect::from_min_max(
                                    Pos2::new(hdr_rect.min.x + 6.0, hdr_rect.min.y + 4.0),
                                    Pos2::new(hdr_rect.min.x + 28.0, hdr_rect.max.y - 4.0),
                                );
                                let back_resp = ui.interact(
                                    back_rect,
                                    popover_id.with("back_btn"),
                                    Sense::click(),
                                );
                                if back_resp.hovered() {
                                    let hover_fill = if app.dark_mode {
                                        Color32::from_rgba_unmultiplied(255, 255, 255, 22)
                                    } else {
                                        Color32::from_rgba_unmultiplied(15, 23, 42, 16)
                                    };
                                    hp.rect_filled(back_rect, 4.0, hover_fill);
                                }
                                hp.text(
                                    back_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "‹",
                                    FontId::proportional(14.5),
                                    p.text_main,
                                );
                                if back_resp.clicked() {
                                    next_step = Some(crate::CatalogPickerStep::Manufacturers);
                                }
                                let sys_hdr = if is_de {
                                    format!("{} SYSTEME", active_mfg.to_uppercase())
                                } else {
                                    format!("{} SYSTEMS", active_mfg.to_uppercase())
                                };
                                let title_clip = Rect::from_min_max(
                                    Pos2::new(back_rect.max.x + 4.0, hdr_rect.min.y),
                                    Pos2::new(hdr_rect.max.x - 10.0, hdr_rect.max.y),
                                );
                                ui.painter_at(title_clip).text(
                                    Pos2::new(title_clip.min.x, hdr_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    sys_hdr,
                                    FontId::proportional(10.5),
                                    p.text_muted,
                                );
                            }
                            crate::CatalogPickerStep::RoadScales => {
                                let hdr_txt = if is_de {
                                    "MASSSTAB WÄHLEN"
                                } else {
                                    "SELECT SCALE"
                                };
                                hp.text(
                                    Pos2::new(hdr_rect.min.x + 12.0, hdr_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    hdr_txt,
                                    FontId::proportional(10.5),
                                    p.text_muted,
                                );
                            }
                            crate::CatalogPickerStep::Closed => {}
                        }

                        // Popover List Body
                        Frame::new().inner_margin(Margin::same(4)).show(ui, |ui| {
                            ui.set_min_width((popover_w - 8.0).max(180.0));
                            ui.set_max_width((popover_w - 8.0).max(180.0));
                            egui::ScrollArea::vertical()
                                .id_salt(popover_id.with("scroll"))
                                .max_height(290.0)
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    match &app.catalog_picker_step {
                                        crate::CatalogPickerStep::Manufacturers => {
                                            let fav_lbl = if is_de {
                                                "★  Meine Favoriten"
                                            } else {
                                                "★  My Favorites"
                                            };
                                            let fav_badge = fav_count.to_string();
                                            if draw_popover_row(
                                                ui,
                                                p,
                                                app.dark_mode,
                                                fav_lbl,
                                                None,
                                                Some(&fav_badge),
                                                show_favorites,
                                                Some(favorite_accent(app.dark_mode, Color32::from_rgb(245, 158, 11))),
                                            ) {
                                                next_cat_filter = Some("fav".into());
                                                next_step = Some(crate::CatalogPickerStep::Closed);
                                            }
                                            draw_popover_divider(ui, app.dark_mode);

                                            let active_mfr = app.catalogs[cur_sel_idx]
                                                .manufacturer
                                                .as_deref()
                                                .filter(|s| !s.is_empty())
                                                .unwrap_or("Other");
                                            for mfr in collect_track_manufacturers_in_order(&app.catalogs, &indices) {
                                                let is_active = !show_favorites && mfr == active_mfr;
                                                if draw_popover_row(
                                                    ui,
                                                    p,
                                                    app.dark_mode,
                                                    &mfr,
                                                    None,
                                                    Some("›"),
                                                    is_active,
                                                    None,
                                                ) {
                                                    next_step = Some(crate::CatalogPickerStep::Systems(mfr));
                                                }
                                            }
                                            draw_popover_divider(ui, app.dark_mode);
                                            let imp_lbl = if is_de {
                                                "+  Katalog importieren (JSON)..."
                                            } else {
                                                "+  Import Catalog (JSON)..."
                                            };
                                            if draw_popover_row(
                                                ui,
                                                p,
                                                app.dark_mode,
                                                imp_lbl,
                                                None,
                                                None,
                                                false,
                                                Some(p.primary),
                                            ) {
                                                trigger_import_catalog = true;
                                                next_step = Some(crate::CatalogPickerStep::Closed);
                                            }
                                            let exp_lbl = if is_de {
                                                "⬇  Aktiven Katalog exportieren (JSON)..."
                                            } else {
                                                "⬇  Export Active Catalog (JSON)..."
                                            };
                                            if draw_popover_row(
                                                ui,
                                                p,
                                                app.dark_mode,
                                                exp_lbl,
                                                None,
                                                None,
                                                false,
                                                Some(p.text_sub),
                                            ) {
                                                trigger_export_catalog = true;
                                                next_step = Some(crate::CatalogPickerStep::Closed);
                                            }
                                        }
                                        crate::CatalogPickerStep::Systems(active_mfg) => {
                                            for &idx in &indices {
                                                let c = &app.catalogs[idx];
                                                let mfr = c
                                                    .manufacturer
                                                    .as_deref()
                                                    .filter(|s| !s.is_empty())
                                                    .unwrap_or("Other");
                                                if mfr != active_mfg {
                                                    continue;
                                                }
                                                let sys_title = c
                                                    .system
                                                    .as_deref()
                                                    .filter(|s| !s.is_empty())
                                                    .unwrap_or(&c.name);
                                                let sub = format!(
                                                    "{} • {} mm",
                                                    c.scale,
                                                    format_gauge_mm(c.gauge.unwrap_or(16.5))
                                                );
                                                let is_sel = !show_favorites && idx == cur_sel_idx;
                                                if draw_popover_row(
                                                    ui,
                                                    p,
                                                    app.dark_mode,
                                                    sys_title,
                                                    Some(&sub),
                                                    if is_sel { Some("✔") } else { None },
                                                    is_sel,
                                                    None,
                                                ) {
                                                    cur_sel_idx = idx;
                                                    if show_favorites {
                                                        next_cat_filter = Some("all".into());
                                                    }
                                                    next_step = Some(crate::CatalogPickerStep::Closed);
                                                }
                                            }
                                            draw_popover_divider(ui, app.dark_mode);
                                            let imp_lbl = if is_de {
                                                "+  Katalog importieren (JSON)..."
                                            } else {
                                                "+  Import Catalog (JSON)..."
                                            };
                                            if draw_popover_row(
                                                ui,
                                                p,
                                                app.dark_mode,
                                                imp_lbl,
                                                None,
                                                None,
                                                false,
                                                Some(p.primary),
                                            ) {
                                                trigger_import_catalog = true;
                                                next_step = Some(crate::CatalogPickerStep::Closed);
                                            }
                                        }
                                        crate::CatalogPickerStep::RoadScales => {
                                            let mut listed = Vec::new();
                                            for sc in ["H0", "TT", "N", "Z", "0", "1", "G"] {
                                                let found_idx = indices.iter().copied().find(|&idx| {
                                                    let c = &app.catalogs[idx];
                                                    c.id.eq_ignore_ascii_case(&format!("generic-streets-{sc}"))
                                                        || c.scale.eq_ignore_ascii_case(sc)
                                                });
                                                let Some(idx) = found_idx else {
                                                    continue;
                                                };
                                                listed.push(idx);
                                                let is_sel = idx == cur_sel_idx;
                                                let title = if is_de {
                                                    format!("Spur {sc}")
                                                } else {
                                                    format!("Scale {sc}")
                                                };
                                                let desc = scale_description_for_road(sc, is_de);
                                                if draw_popover_row(
                                                    ui,
                                                    p,
                                                    app.dark_mode,
                                                    &title,
                                                    Some(desc),
                                                    if is_sel { Some("✔") } else { None },
                                                    is_sel,
                                                    None,
                                                ) {
                                                    cur_sel_idx = idx;
                                                    next_step = Some(crate::CatalogPickerStep::Closed);
                                                }
                                            }
                                            for &idx in &indices {
                                                if listed.contains(&idx) {
                                                    continue;
                                                }
                                                let c = &app.catalogs[idx];
                                                let is_sel = idx == cur_sel_idx;
                                                let desc = scale_description_for_road(&c.scale, is_de);
                                                if draw_popover_row(
                                                    ui,
                                                    p,
                                                    app.dark_mode,
                                                    &c.name,
                                                    Some(desc),
                                                    if is_sel { Some("✔") } else { None },
                                                    is_sel,
                                                    None,
                                                ) {
                                                    cur_sel_idx = idx;
                                                    next_step = Some(crate::CatalogPickerStep::Closed);
                                                }
                                            }
                                            draw_popover_divider(ui, app.dark_mode);
                                            let imp_lbl = if is_de {
                                                "+  Katalog importieren (JSON)..."
                                            } else {
                                                "+  Import Catalog (JSON)..."
                                            };
                                            if draw_popover_row(
                                                ui,
                                                p,
                                                app.dark_mode,
                                                imp_lbl,
                                                None,
                                                None,
                                                false,
                                                Some(p.primary),
                                            ) {
                                                trigger_import_catalog = true;
                                                next_step = Some(crate::CatalogPickerStep::Closed);
                                            }
                                            let exp_lbl = if is_de {
                                                "⬇  Aktiven Katalog exportieren (JSON)..."
                                            } else {
                                                "⬇  Export Active Catalog (JSON)..."
                                            };
                                            if draw_popover_row(
                                                ui,
                                                p,
                                                app.dark_mode,
                                                exp_lbl,
                                                None,
                                                None,
                                                false,
                                                Some(p.text_sub),
                                            ) {
                                                trigger_export_catalog = true;
                                                next_step = Some(crate::CatalogPickerStep::Closed);
                                            }
                                        }
                                        crate::CatalogPickerStep::Closed => {}
                                    }
                                });
                        });
                    });
            });

        let area_rect = area_resp.response.rect;
        if next_step.is_none()
            && ui.ctx().input(|i| {
                i.key_pressed(egui::Key::Escape)
                    || (i.pointer.any_pressed()
                        && i.pointer
                            .interact_pos()
                            .map_or(false, |pos| !trigger_rect.contains(pos) && !area_rect.contains(pos)))
            })
        {
            next_step = Some(crate::CatalogPickerStep::Closed);
        }
    }

    if let Some(step) = next_step {
        app.catalog_picker_step = step;
    }
    if let Some(cf) = next_cat_filter {
        app.catalog_category_filter = cf;
    }
    if roads_only {
        app.selected_road_catalog_idx = cur_sel_idx;
    } else {
        app.selected_catalog_idx = cur_sel_idx;
    }

    ui.add_space(6.0);
    let search_w = ui.available_width();
    ui.add(
        egui::TextEdit::singleline(&mut app.catalog_search)
            .hint_text(t.search_hint)
            .desired_width(search_w),
    );
    let cat_idx = cur_sel_idx;
    if cat_idx != prev_cat_idx {
        if !roads_only && app.layout.pieces.is_empty() {
            let new_scale = app.catalogs[cat_idx].scale.clone();
            if !new_scale.is_empty() {
                app.layout.scale = new_scale.clone();
                app.layout.feed_voltage = Some(match new_scale.trim().to_uppercase().as_str() {
                    "Z" => 10.0,
                    "N" => 12.0,
                    "TT" | "H0M" => 14.0,
                    "1" => 18.0,
                    "G" => 24.0,
                    _ => 16.0,
                });
                app.model_scale = crate::model_tools::parse_scale_code(&app.layout.scale).into();
            }
        }
        app.save_current_settings();
    }
    if trigger_import_catalog {
        app.import_catalog_dialog();
    }
    if trigger_export_catalog {
        app.export_catalog_dialog(cat_idx);
    }

    ui.add_space(4.0);
    let is_de = app.lang == Lang::De;
    let fav_count = app.app_settings.favorite_tracks.len();
    let fav_pill_label = if fav_count > 0 {
        format!("★ {fav_count}")
    } else {
        "★".to_string()
    };
    let pills: &[(&str, &str)] = if roads_only {
        &[
            ("fav", fav_pill_label.as_str()),
            ("all", if is_de { "Alle" } else { "All" }),
            ("straights", if is_de { "Geraden" } else { "Straights" }),
            ("curves", if is_de { "Kurven" } else { "Curves" }),
            ("crossings", if is_de { "Kreuz." } else { "Crossings" }),
            ("accessories", if is_de { "Zubehör" } else { "Special" }),
        ]
    } else {
        &[
            ("fav", fav_pill_label.as_str()),
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
                favorite_accent(app.dark_mode, Color32::from_rgb(250, 204, 21))
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
    let mut dragged_entry_opt: Option<railbench_core::types::CatalogEntry> = None;
    let mut clicked_entry_opt: Option<railbench_core::types::CatalogEntry> = None;
    let mut toggled_fav: Option<(String, String)> = None;
    let mut toggled_feeder_id: Option<String> = None;
    let mut toggled_iso_id: Option<String> = None;

    let active_cat_id = app.catalogs[cat_idx].id.clone();
    // Build display list: when in "fav" filter, show favorites across ALL matching catalogs (Tracks or Roads),
    // otherwise sort entries within categories naturally by article number.
    let mut items: Vec<(String, String, &railbench_core::types::CatalogEntry)> = Vec::new();
    if cat_filter == "fav" {
        for c in &app.catalogs {
            if (c.is_road == Some(true)) != roads_only {
                continue;
            }
            for e in &c.entries {
                if app.is_favorite_track_entry(&c.id, &e.id) {
                    items.push((c.id.clone(), c.name.clone(), e));
                }
            }
        }
        items.sort_by(|a, b| crate::catalog::compare_catalog_entries_by_number(a.2, b.2));
    } else {
        let mut cat_entries: Vec<&railbench_core::types::CatalogEntry> = app.catalogs[cat_idx].entries.iter().collect();
        cat_entries.sort_by(|a, b| {
            a.category
                .cmp(&b.category)
                .then_with(|| crate::catalog::compare_catalog_entries_by_number(a, b))
        });
        for e in cat_entries {
            items.push((active_cat_id.clone(), e.category.clone(), e));
        }
    }

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let mut last_cat = String::new();
        for (item_cat_id, group_header, entry) in items {
            let is_fav = app.is_favorite_track_entry(&item_cat_id, &entry.id);
            let is_buf = crate::catalog::is_buffer_stop_entry(entry);
            let kind_matches_pill = match cat_filter.as_str() {
                "fav" => true,
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
                            "buffer" | "buffer-stop" | "accessory" | "turntable" | "transfer-table" | "feeder" | "uncoupler" | "isolating" | "isolator"
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
                && !contains_ignore_case(&entry.name, &filter)
                && !contains_ignore_case(entry.name_en.as_deref().unwrap_or(""), &filter)
                && !contains_ignore_case(&entry.description, &filter)
                && !contains_ignore_case(&entry.category, &filter)
            {
                continue;
            }
            if group_header != last_cat {
                if !last_cat.is_empty() {
                    ui.add_space(6.0);
                }
                ui.label(
                    RichText::new(group_header.to_uppercase())
                        .strong()
                        .size(10.5)
                        .color(p.text_muted),
                );
                last_cat = group_header;
            }

            let icon_kind = if is_buf { "buffer" } else { entry.kind.as_str() };
            let bp = crate::blueprint_colors::get_track_blueprint_color(&entry.id);
            let is_road_entry = entry.is_road == Some(true);
            let accent = if is_road_entry {
                Color32::from_rgb(100, 116, 139)
            } else {
                bp.color32()
            };

            let dim_text = if crate::is_turnout_kind(&entry.kind) {
                if let (Some(l), Some(ba)) = (entry.length, entry.branch_angle) {
                    format!("{l:.0} mm • {ba:.1}°")
                } else if let Some(ba) = entry.branch_angle {
                    format!("{ba:.1}°")
                } else if let Some(l) = entry.length {
                    format!("{l:.1} mm")
                } else {
                    String::new()
                }
            } else if let (Some(r), Some(a)) = (entry.radius, entry.angle) {
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
                                    if !is_road_entry {
                                        let (dot_r, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                                        ui.painter().circle_filled(dot_r.center(), 4.0, bp.color32());
                                    }
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
                                            favorite_accent(app.dark_mode, Color32::from_rgb(250, 204, 21))
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
                                            if icon_button_sized(
                                                ui,
                                                Icon::Isolator,
                                                egui::vec2(20.0, 20.0),
                                                iso_enabled,
                                                p.bg_input,
                                                Color32::from_rgb(220, 38, 38),
                                                Color32::from_rgb(220, 38, 38),
                                                p.text_muted,
                                                iso_tip,
                                            )
                                            .clicked()
                                            {
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
                                            if icon_button_sized(
                                                ui,
                                                Icon::Zap,
                                                egui::vec2(20.0, 20.0),
                                                power_enabled,
                                                p.bg_input,
                                                Color32::from_rgb(217, 119, 6),
                                                Color32::from_rgb(245, 158, 11),
                                                p.text_muted,
                                                feed_tip,
                                            )
                                            .clicked()
                                            {
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
                toggled_fav = Some((item_cat_id, entry.id.clone()));
            } else if feeder_toggle_clicked {
                toggled_feeder_id = Some(entry.id.clone());
            } else if iso_toggle_clicked {
                toggled_iso_id = Some(entry.id.clone());
            } else if plus_clicked || card_resp.clicked() {
                clicked_entry_opt = Some(entry.clone());
            } else if card_resp.drag_started_by(egui::PointerButton::Primary) {
                dragged_entry_opt = Some(entry.clone());
            }
        }
    });

    if let Some((fcid, ftid)) = toggled_fav {
        app.toggle_favorite_track_entry(&fcid, &ftid);
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
    if let Some(entry_clone) = dragged_entry_opt {
        app.dragging_catalog_entry = Some(entry_clone);
    }
    if let Some(entry_clone) = clicked_entry_opt {
        app.place_catalog_entry(&entry_clone);
    }
}
