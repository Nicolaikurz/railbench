use super::*;

pub(super) fn draw_bom_tab(ui: &mut egui::Ui, app: &mut RailbenchApp) {
    let t = tr(app.lang);
    let p = palette(app.dark_mode);
    let is_de = app.lang == Lang::De;
    let sum = get_piece_list_summary(&app.layout, &app.catalogs, is_de);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} ({})", t.tab_bom.to_uppercase(), sum.total_objects))
                .strong()
                .size(11.0)
                .color(p.text_muted),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("PDF").size(10.5).color(Color32::WHITE))
                        .fill(Color32::from_rgb(51, 65, 85))
                        .corner_radius(CornerRadius::same(6)),
                )
                .on_hover_text(if is_de { "Bauplan-Export & Drucken" } else { "Blueprint Export & Print" })
                .clicked()
            {
                app.show_export_dialog = true;
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("CSV").size(10.5).color(Color32::WHITE))
                        .fill(p.primary_hover)
                        .corner_radius(CornerRadius::same(6)),
                )
                .on_hover_text(t.export_csv)
                .clicked()
            {
                app.export_bom_csv_dialog();
            }
        });
    });
    ui.add_space(6.0);

    // Compact summary metrics grid (tracks meters, roads meters, models)
    Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(8))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::Grid::new("bom_metrics_grid")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label(RichText::new(if is_de { "Gleise:" } else { "Tracks:" }).size(11.0).color(p.text_muted));
                    ui.label(
                        RichText::new(format!("{} ({:.2} m)", sum.tracks.total_pieces, sum.tracks.total_length_meters))
                            .strong()
                            .size(11.5)
                            .color(p.primary),
                    );
                    ui.end_row();
                    ui.label(RichText::new(if is_de { "Strassen:" } else { "Roads:" }).size(11.0).color(p.text_muted));
                    ui.label(
                        RichText::new(format!("{} ({:.2} m)", sum.roads.total_pieces, sum.roads.total_length_meters))
                            .strong()
                            .size(11.5)
                            .color(p.text_main),
                    );
                    ui.end_row();
                    ui.label(RichText::new(if is_de { "Modelle:" } else { "Models:" }).size(11.0).color(p.text_muted));
                    ui.label(
                        RichText::new(format!("{} | Spur {}", sum.total_buildings, app.layout.scale))
                            .strong()
                            .size(11.5)
                            .color(p.text_main),
                    );
                    ui.end_row();
                    ui.label(RichText::new(if is_de { "Gesamt:" } else { "Total:" }).size(11.0).color(p.text_muted));
                    ui.label(
                        RichText::new(format!("{} ({:.2} m)", sum.total_pieces, sum.total_length_meters))
                            .strong()
                            .size(11.5)
                            .color(p.text_main),
                    );
                    ui.end_row();
                });
        });

    ui.add_space(6.0);

    // Budget & Personal Track Stock Card
    let budget = app.inventory.compute_bom_budget(&sum);
    let mut inv_dirty = false;
    Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(8))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if is_de { "LAGER & KOSTEN" } else { "STOCK & BUDGET" })
                        .strong()
                        .size(10.5)
                        .color(p.text_muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("bom_currency")
                        .width(56.0)
                        .selected_text(&app.inventory.currency)
                        .show_ui(ui, |ui| {
                            for cur in ["EUR", "USD", "CHF", "GBP"] {
                                if ui.selectable_value(&mut app.inventory.currency, cur.into(), cur).changed() {
                                    inv_dirty = true;
                                }
                            }
                        });
                });
            });
            ui.add_space(3.0);
            let sym = app.inventory.currency_symbol().to_string();
            egui::Grid::new("bom_budget_grid")
                .num_columns(2)
                .spacing([10.0, 3.0])
                .show(ui, |ui| {
                    ui.label(RichText::new(if is_de { "Vorhanden:" } else { "In Stock:" }).size(10.5).color(p.text_muted));
                    ui.label(RichText::new(format!("{} / {}", budget.total_owned_used, budget.total_needed)).strong().size(11.0).color(Color32::from_rgb(34, 197, 94)));
                    ui.end_row();
                    ui.label(RichText::new(if is_de { "Zuzukaufen:" } else { "To Buy:" }).size(10.5).color(p.text_muted));
                    ui.label(
                        RichText::new(format!("{} Stk ({:.2} {sym})", budget.total_to_buy, budget.remaining_buy_cost))
                            .strong()
                            .size(11.0)
                            .color(if budget.total_to_buy > 0 { Color32::from_rgb(245, 158, 11) } else { Color32::from_rgb(34, 197, 94) }),
                    );
                    ui.end_row();
                    ui.label(RichText::new(if is_de { "Gesamtwert:" } else { "Total Value:" }).size(10.5).color(p.text_muted));
                    ui.label(RichText::new(format!("{:.2} {sym}", budget.total_layout_cost)).strong().size(11.0).color(p.text_main));
                    ui.end_row();
                });
        });
    ui.add_space(6.0);

    // Category filter tabs
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        for (cid, lbl) in [
            ("all", if is_de { "Alle" } else { "All" }),
            ("tracks", if is_de { "Gleise" } else { "Tracks" }),
            ("roads", if is_de { "Strassen" } else { "Roads" }),
            ("buildings", if is_de { "Gebaeude" } else { "Models" }),
        ] {
            let active = app.bom_category == cid;
            let fill = if active { p.primary_hover } else { p.bg_card };
            let tc = if active { Color32::WHITE } else { p.text_muted };
            if ui
                .add(
                    egui::Button::new(RichText::new(lbl).size(11.0).color(tc))
                        .fill(fill)
                        .stroke(Stroke::new(1.0_f32, if active { p.primary } else { p.border_main }))
                        .corner_radius(CornerRadius::same(6)),
                )
                .clicked()
            {
                app.bom_category = cid.into();
            }
        }
    });
    ui.add_space(6.0);

    let show_tracks = matches!(app.bom_category.as_str(), "all" | "tracks");
    let show_roads = matches!(app.bom_category.as_str(), "all" | "roads");
    let show_models = matches!(app.bom_category.as_str(), "all" | "buildings");

    egui::ScrollArea::vertical().show(ui, |ui| {
        if show_tracks && !sum.tracks.items.is_empty() {
            ui.label(
                RichText::new(if is_de {
                    format!("GLEISE & WEICHEN ({} | {:.2} m)", sum.tracks.total_pieces, sum.tracks.total_length_meters)
                } else {
                    format!("RAILWAY TRACKS ({} | {:.2} m)", sum.tracks.total_pieces, sum.tracks.total_length_meters)
                })
                .strong()
                .size(10.0)
                .color(p.text_muted),
            );
            ui.add_space(3.0);
            for it in &sum.tracks.items {
                inv_dirty |= draw_bom_piece_card(ui, p, it, false, &mut app.inventory, is_de);
            }
            ui.add_space(6.0);
        }

        if show_roads && !sum.roads.items.is_empty() {
            ui.label(
                RichText::new(if is_de {
                    format!("STRASSEN & WEGE ({} | {:.2} m)", sum.roads.total_pieces, sum.roads.total_length_meters)
                } else {
                    format!("ROADS & STREETS ({} | {:.2} m)", sum.roads.total_pieces, sum.roads.total_length_meters)
                })
                .strong()
                .size(10.0)
                .color(p.text_muted),
            );
            ui.add_space(3.0);
            for it in &sum.roads.items {
                inv_dirty |= draw_bom_piece_card(ui, p, it, true, &mut app.inventory, is_de);
            }
            ui.add_space(6.0);
        }

        if show_models && !sum.buildings.is_empty() {
            ui.label(
                RichText::new(if is_de {
                    format!("GEBAEUDE & 3D-OBJEKTE ({})", sum.total_buildings)
                } else {
                    format!("BUILDINGS & 3D OBJECTS ({})", sum.total_buildings)
                })
                .strong()
                .size(10.0)
                .color(p.text_muted),
            );
            ui.add_space(3.0);
            for bm in &sum.buildings {
                Frame::new()
                    .fill(p.bg_card)
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(9, 6))
                    .stroke(Stroke::new(1.0_f32, p.border_main))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{}x", bm.count)).strong().color(p.primary));
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.label(RichText::new(&bm.name).strong().size(12.0).color(p.text_main));
                                ui.label(
                                    RichText::new(format!("{} ({} mm)", bm.category, bm.dimensions))
                                        .size(10.5)
                                        .color(p.text_muted),
                                );
                            });
                        });
                    });
            }
        }

        if sum.total_objects == 0 {
            ui.label(
                RichText::new(if is_de {
                    "Noch keine Gleise oder Modelle im Plan."
                } else {
                    "No tracks or models placed yet."
                })
                .size(11.5)
                .color(p.text_muted),
            );
        }
    });
    if inv_dirty {
        app.inventory.save_to_disk();
    }
}

pub(super) fn draw_bom_piece_card(
    ui: &mut egui::Ui,
    p: crate::theme::RailbenchPalette,
    it: &crate::export::PieceSummaryItem,
    is_road: bool,
    inv: &mut crate::inventory_store::TrackInventoryStore,
    is_de: bool,
) -> bool {
    let mut dirty = false;
    let mut owned = inv.owned(&it.catalog_id);
    let mut price = inv.unit_price(&it.catalog_id);
    let to_buy = it.count.saturating_sub(owned);
    let sym = inv.currency_symbol().to_string();

    Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(9, 6))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{}x", it.count)).strong().color(p.primary));
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.horizontal(|ui| {
                        if !is_road {
                            let bp = crate::blueprint_colors::get_track_blueprint_color(&it.catalog_id);
                            Frame::new()
                                .fill(bp.badge_bg_color32())
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(6, 2))
                                .stroke(Stroke::new(1.0_f32, bp.border_color32()))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&it.catalog_id)
                                            .strong()
                                            .size(11.0)
                                            .color(Color32::WHITE),
                                    );
                                });
                        } else {
                            ui.label(RichText::new(&it.catalog_id).strong().size(12.0).color(p.text_main));
                        }
                        if it.total_length_meters > 0.0 {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{:.2} m", it.total_length_meters))
                                        .size(10.5)
                                        .color(p.text_muted),
                                );
                            });
                        }
                    });
                    if !it.name.is_empty() && it.name != it.catalog_id {
                        ui.label(RichText::new(&it.name).size(10.5).color(p.text_muted));
                    }
                    if !it.specification.is_empty() {
                        ui.label(RichText::new(&it.specification).size(10.0).color(p.text_muted));
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(if is_de { "Lager:" } else { "Stock:" }).size(10.0).color(p.text_muted));
                        if ui
                            .add(egui::DragValue::new(&mut owned).range(0..=9999).speed(0.2))
                            .changed()
                        {
                            dirty = true;
                        }
                        ui.label(RichText::new(format!("{sym}:")).size(10.0).color(p.text_muted));
                        if ui
                            .add(egui::DragValue::new(&mut price).range(0.0..=9999.0).speed(0.1).max_decimals(2))
                            .changed()
                        {
                            dirty = true;
                        }
                        let st_col = if to_buy == 0 {
                            Color32::from_rgb(34, 197, 94)
                        } else {
                            Color32::from_rgb(245, 158, 11)
                        };
                        let st_txt = if to_buy == 0 {
                            if is_de { "OK".into() } else { "OK".into() }
                        } else if is_de {
                            format!("+{to_buy}")
                        } else {
                            format!("Buy {to_buy}")
                        };
                        ui.label(RichText::new(st_txt).strong().size(10.0).color(st_col));
                    });
                });
            });
        });
    if dirty {
        inv.set_entry(&it.catalog_id, owned, price);
    }
    dirty
}

