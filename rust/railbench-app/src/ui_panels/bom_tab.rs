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
        let mut any_visible = false;

        if show_tracks && !sum.tracks.items.is_empty() {
            any_visible = true;
            ui.label(
                RichText::new(if is_de {
                    format!("GLEISE & WEICHEN ({})", sum.tracks.total_pieces)
                } else {
                    format!("RAILWAY TRACKS ({})", sum.tracks.total_pieces)
                })
                .strong()
                .size(10.0)
                .color(p.text_muted),
            );
            ui.add_space(3.0);
            for it in &sum.tracks.items {
                draw_bom_piece_row(ui, p, it, false);
            }
            ui.add_space(6.0);
        }

        if show_roads && !sum.roads.items.is_empty() {
            any_visible = true;
            ui.label(
                RichText::new(if is_de {
                    format!("STRASSEN & WEGE ({})", sum.roads.total_pieces)
                } else {
                    format!("ROADS & STREETS ({})", sum.roads.total_pieces)
                })
                .strong()
                .size(10.0)
                .color(p.text_muted),
            );
            ui.add_space(3.0);
            for it in &sum.roads.items {
                draw_bom_piece_row(ui, p, it, true);
            }
            ui.add_space(6.0);
        }

        if show_models && !sum.buildings.is_empty() {
            any_visible = true;
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
                draw_bom_item_row(ui, p, None, &bm.name, bm.count, false);
            }
        }

        if !any_visible {
            let empty_msg = if sum.total_objects == 0 {
                if is_de {
                    "Noch keine Gleise oder Modelle im Plan."
                } else {
                    "No tracks or models placed yet."
                }
            } else if is_de {
                "Keine Elemente in dieser Kategorie."
            } else {
                "No items in this category."
            };
            ui.label(RichText::new(empty_msg).size(11.5).color(p.text_muted));
        }
    });
}

fn draw_bom_piece_row(
    ui: &mut egui::Ui,
    p: crate::theme::RailbenchPalette,
    it: &crate::export::PieceSummaryItem,
    is_road: bool,
) {
    let display_name = if !it.name.is_empty() && it.name != it.catalog_id {
        it.name.as_str()
    } else {
        ""
    };
    draw_bom_item_row(ui, p, Some(&it.catalog_id), display_name, it.count, is_road);
}

fn draw_bom_item_row(
    ui: &mut egui::Ui,
    p: crate::theme::RailbenchPalette,
    article_id: Option<&str>,
    name: &str,
    count: usize,
    is_road: bool,
) {
    Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(9, 6))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            let row_w = ui.available_width().max(120.0);
            let count_w = 38.0_f32;
            let gap = 6.0_f32;
            let main_w = (row_w - count_w - gap).max(60.0);
            ui.allocate_ui_with_layout(
                egui::vec2(row_w, 0.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    ui.allocate_ui_with_layout(
                        egui::vec2(main_w, 0.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            if let Some(id) = article_id {
                                let (badge_bg, badge_border) = if is_road {
                                    (Color32::from_rgb(51, 65, 85), Color32::from_rgb(100, 116, 139))
                                } else {
                                    let bp = crate::blueprint_colors::get_track_blueprint_color(id);
                                    (bp.badge_bg_color32(), bp.border_color32())
                                };
                                Frame::new()
                                    .fill(badge_bg)
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(Margin::symmetric(6, 2))
                                    .stroke(Stroke::new(1.0_f32, badge_border))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(id)
                                                .strong()
                                                .size(11.0)
                                                .color(Color32::WHITE),
                                        );
                                    });
                            }
                            if !name.is_empty() {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(name).size(11.5).color(p.text_main),
                                    )
                                    .truncate(),
                                );
                            }
                        },
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(count_w, 0.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(format!("{count}x"))
                                    .strong()
                                    .size(11.5)
                                    .color(p.primary),
                            );
                        },
                    );
                },
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bom_tab_renders_used_items_without_stock_or_budget_in_de_and_en() {
        let mut app = crate::RailbenchApp::default();
        let inv_before = app.inventory.clone();
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 640.0));

        // 1. Populated layout (tracks, road, building)
        let e24188 = crate::catalog::find_entry(&app.catalogs, "24188")
            .expect("24188")
            .clone();
        let road_entry = app
            .catalogs
            .iter()
            .flat_map(|c| c.entries.iter())
            .find(|e| e.is_road == Some(true))
            .expect("road entry")
            .clone();
        app.place_catalog_entry(&e24188);
        app.place_catalog_entry(&road_entry);
        app.place_builtin_model("station", "Station", 320.0, 140.0, 130.0);

        for lang in [crate::i18n::Lang::De, crate::i18n::Lang::En] {
            app.lang = lang;
            for cat in ["all", "tracks", "roads", "buildings"] {
                app.bom_category = cat.into();
                let mut raw = egui::RawInput::default();
                raw.screen_rect = Some(screen);
                let _ = ctx.run(raw, |c| {
                    egui::SidePanel::left("bom_test_panel").show(c, |ui| draw_bom_tab(ui, &mut app));
                });
            }
        }

        // 2. Empty layout
        app.layout = crate::make_empty_layout();
        app.bom_category = "all".into();
        let mut raw = egui::RawInput::default();
        raw.screen_rect = Some(screen);
        let _ = ctx.run(raw, |c| {
            egui::SidePanel::left("bom_test_empty").show(c, |ui| draw_bom_tab(ui, &mut app));
        });

        assert_eq!(app.inventory, inv_before);
    }
}
