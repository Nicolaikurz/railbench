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
        let list_w = ui.available_width().max(180.0);
        ui.set_width(list_w);
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
                draw_bom_piece_row(ui, p, it, BomBadgeKind::Track);
                ui.add_space(3.0);
            }
            ui.add_space(3.0);
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
                draw_bom_piece_row(ui, p, it, BomBadgeKind::Road);
                ui.add_space(3.0);
            }
            ui.add_space(3.0);
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
                draw_bom_item_row(ui, p, BomBadgeKind::Model, "3D", &bm.name, bm.count);
                ui.add_space(3.0);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BomBadgeKind {
    Track,
    Road,
    Model,
}

const BOM_BADGE_W: f32 = 72.0;
const BOM_COUNT_W: f32 = 44.0;
const BOM_COL_GAP: f32 = 6.0;
const BOM_ROW_H: f32 = 22.0;

/// Compute the fixed-alignment `(badge_rect, name_rect, count_rect)` columns for a Part List row.
fn bom_row_column_rects(row_rect: egui::Rect) -> (egui::Rect, egui::Rect, egui::Rect) {
    let count_w = BOM_COUNT_W.min(row_rect.width());
    let badge_w = BOM_BADGE_W.min((row_rect.width() - count_w - BOM_COL_GAP).max(0.0));
    let badge_rect = egui::Rect::from_min_max(
        row_rect.min,
        egui::pos2(row_rect.min.x + badge_w, row_rect.max.y),
    );
    let count_rect = egui::Rect::from_min_max(
        egui::pos2(row_rect.max.x - count_w, row_rect.min.y),
        row_rect.max,
    );
    let name_left = (badge_rect.max.x + BOM_COL_GAP).min(count_rect.min.x);
    let name_right = (count_rect.min.x - BOM_COL_GAP).max(name_left);
    let name_rect = egui::Rect::from_min_max(
        egui::pos2(name_left, row_rect.min.y),
        egui::pos2(name_right, row_rect.max.y),
    );
    (badge_rect, name_rect, count_rect)
}

fn draw_bom_piece_row(
    ui: &mut egui::Ui,
    p: crate::theme::RailbenchPalette,
    it: &crate::export::PieceSummaryItem,
    kind: BomBadgeKind,
) {
    let display_name = if !it.name.is_empty() && it.name != it.catalog_id {
        it.name.as_str()
    } else {
        ""
    };
    draw_bom_item_row(ui, p, kind, &it.catalog_id, display_name, it.count);
}

fn draw_bom_item_row(
    ui: &mut egui::Ui,
    p: crate::theme::RailbenchPalette,
    kind: BomBadgeKind,
    badge_text: &str,
    name: &str,
    count: usize,
) {
    Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(8, 5))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            let inner_w = ui.available_width().max(160.0);
            let (row_rect, row_resp) =
                ui.allocate_exact_size(egui::vec2(inner_w, BOM_ROW_H), egui::Sense::hover());
            let (badge_rect, name_rect, count_rect) = bom_row_column_rects(row_rect);

            let (badge_bg, badge_border) = match kind {
                BomBadgeKind::Road => (
                    Color32::from_rgb(51, 65, 85),
                    Color32::from_rgb(100, 116, 139),
                ),
                BomBadgeKind::Model => (
                    Color32::from_rgb(67, 56, 202),
                    Color32::from_rgb(129, 140, 248),
                ),
                BomBadgeKind::Track => {
                    let bp = crate::blueprint_colors::get_track_blueprint_color(badge_text);
                    (bp.badge_bg_color32(), bp.border_color32())
                }
            };

            ui.painter().rect(
                badge_rect,
                CornerRadius::same(4),
                badge_bg,
                Stroke::new(1.0_f32, badge_border),
                egui::StrokeKind::Inside,
            );
            let badge_inner = badge_rect.shrink2(egui::vec2(4.0, 0.0));
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(badge_inner)
                    .layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight)),
                |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(badge_text)
                                .strong()
                                .size(10.5)
                                .color(Color32::WHITE),
                        )
                        .truncate(),
                    );
                },
            );

            if !name.is_empty() && name_rect.width() > 4.0 {
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(name_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(name).size(11.5).color(p.text_main))
                                .truncate(),
                        );
                    },
                );
            }

            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(count_rect)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
                |ui| {
                    ui.label(
                        RichText::new(format!("{count}x"))
                            .monospace()
                            .strong()
                            .size(11.5)
                            .color(p.primary),
                    );
                },
            );

            let tooltip = if name.is_empty() {
                format!("{badge_text} - {count}x")
            } else {
                format!("{badge_text} - {name} ({count}x)")
            };
            row_resp.on_hover_text(tooltip);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bom_row_columns_align_uniformly_without_overlap() {
        for width in [160.0_f32, 240.0, 320.0, 480.0] {
            let row = egui::Rect::from_min_size(egui::pos2(12.0, 20.0), egui::vec2(width, BOM_ROW_H));
            let (badge, name, count) = bom_row_column_rects(row);
            assert_eq!(badge.min.x, row.min.x);
            assert_eq!(count.max.x, row.max.x);
            assert_eq!(count.width(), BOM_COUNT_W);
            assert!(badge.max.x <= name.min.x);
            assert!(name.max.x <= count.min.x);
            assert_eq!(badge.height(), BOM_ROW_H);
            assert_eq!(name.height(), BOM_ROW_H);
            assert_eq!(count.height(), BOM_ROW_H);
        }
    }

    #[test]
    fn bom_tab_renders_used_items_without_stock_or_budget_in_de_and_en() {
        let mut app = crate::RailbenchApp::default();
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 640.0));

        // 1. Populated layout (short/long track IDs, road, building)
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
        app.place_catalog_entry(&e24188);
        app.place_catalog_entry(&road_entry);
        app.place_builtin_model(
            "station",
            "Very Long Bavarian Countryside Terminal Station Building",
            320.0,
            140.0,
            130.0,
        );

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
    }
}
