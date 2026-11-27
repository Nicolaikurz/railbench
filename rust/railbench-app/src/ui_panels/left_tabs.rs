use super::*;
use eframe::egui::{FontId, Pos2, Rect, Sense};

pub fn draw_left_sidebar(ctx: &egui::Context, app: &mut RailbenchApp) {
    let p = palette(app.dark_mode);
    let min_w = ctx
        .data(|d| d.get_temp::<f32>(egui::Id::new(LEFT_SIDEBAR_MIN_W_KEY)))
        .unwrap_or(320.0)
        .clamp(260.0, 520.0);
    let init_w = app
        .app_settings
        .left_sidebar_width
        .unwrap_or(min_w)
        .clamp(min_w, 520.0);

    let panel_resp = egui::SidePanel::left("left_sidebar")
        .default_width(init_w)
        .width_range(min_w..=520.0)
        .resizable(!app.bars_locked)
        .frame(
            Frame::new()
                .fill(p.bg_sidebar)
                .inner_margin(Margin::same(10))
                .stroke(Stroke::new(1.0_f32, p.border_main)),
        )
        .show(ctx, |ui| {
            match app.sidebar_tab {
                SidebarTab::Tracks => draw_catalog_tab(ui, app, false),
                SidebarTab::Roads => draw_catalog_tab(ui, app, true),
                SidebarTab::Models => draw_models_tab(ui, app),
                SidebarTab::Terrain => draw_terrain_tab(ui, app),
                SidebarTab::Board => draw_board_tab(ui, app),
                SidebarTab::Bom => draw_bom_tab(ui, app),
            }
        });
    let cur_w = panel_resp.response.rect.width().clamp(min_w, 520.0);
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new(LEFT_SIDEBAR_ACTUAL_W_KEY), cur_w);
    });
    if (cur_w - app.app_settings.left_sidebar_width.unwrap_or(min_w)).abs() > 2.0 {
        app.app_settings.left_sidebar_width = Some(cur_w);
        app.save_current_settings();
    }
}

pub(crate) fn road_width_mm_for_scale(scale: &str) -> u32 {
    match scale.trim().to_ascii_uppercase().as_str() {
        "H0" | "HO" => 60,
        "TT" => 45,
        "N" => 35,
        "Z" => 25,
        "0" => 115,
        "1" => 160,
        "G" => 230,
        _ => 60,
    }
}

pub(crate) fn scale_description_for_road(scale: &str, is_de: bool) -> &'static str {
    match scale.trim().to_ascii_uppercase().as_str() {
        "H0" | "HO" => {
            if is_de {
                "1:87 (Fahrbahn ~60 mm)"
            } else {
                "1:87 (Roadway ~60 mm)"
            }
        }
        "TT" => {
            if is_de {
                "1:120 (Fahrbahn ~45 mm)"
            } else {
                "1:120 (Roadway ~45 mm)"
            }
        }
        "N" => {
            if is_de {
                "1:160 (Fahrbahn ~35 mm)"
            } else {
                "1:160 (Roadway ~35 mm)"
            }
        }
        "Z" => {
            if is_de {
                "1:220 (Fahrbahn ~25 mm)"
            } else {
                "1:220 (Roadway ~25 mm)"
            }
        }
        "0" => {
            if is_de {
                "1:45 (Fahrbahn ~115 mm)"
            } else {
                "1:45 (Roadway ~115 mm)"
            }
        }
        "1" => {
            if is_de {
                "1:32 (Fahrbahn ~160 mm)"
            } else {
                "1:32 (Roadway ~160 mm)"
            }
        }
        "G" => {
            if is_de {
                "1:22,5 (Fahrbahn ~230 mm)"
            } else {
                "1:22.5 (Roadway ~230 mm)"
            }
        }
        _ => {
            if is_de {
                "1:87 (Fahrbahn ~60 mm)"
            } else {
                "1:87 (Roadway ~60 mm)"
            }
        }
    }
}

pub(crate) fn format_gauge_mm(gauge: f64) -> String {
    let rounded = gauge.round();
    if (gauge - rounded).abs() < 0.05 {
        format!("{}", rounded as i64)
    } else {
        format!("{gauge:.1}")
    }
}

pub(crate) fn format_catalog_trigger_title(
    cat: &railbench_core::types::Catalog,
    roads_only: bool,
    show_favorites: bool,
    is_de: bool,
) -> String {
    if !roads_only && show_favorites {
        return if is_de {
            "★ Meine Favoriten".into()
        } else {
            "★ My Favorites".into()
        };
    }
    if roads_only {
        return crate::catalog::localized_catalog_name(cat, is_de);
    }
    let mfr = cat
        .manufacturer
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(if is_de { "Sonstige" } else { "Other" });
    let sys = crate::catalog::localized_catalog_system(cat, is_de);
    format!("{mfr} — {sys}")
}

pub(crate) fn format_catalog_trigger_subtitle(
    cat: &railbench_core::types::Catalog,
    roads_only: bool,
    show_favorites: bool,
    fav_count: usize,
    is_de: bool,
) -> String {
    if !roads_only && show_favorites {
        return if is_de {
            format!("{fav_count} gespeicherte Favoriten")
        } else {
            format!("{fav_count} saved favorites")
        };
    }
    if roads_only {
        let w = road_width_mm_for_scale(&cat.scale);
        return if is_de {
            format!("Fahrbahnbreite {w} mm, 2-spurig")
        } else {
            format!("Road width {w} mm, 2-lane")
        };
    }
    let gauge = format_gauge_mm(cat.gauge.unwrap_or(16.5));
    let roadbed = if cat.has_roadbed == Some(false) {
        if is_de { "ohne Bettung" } else { "without roadbed" }
    } else if is_de {
        "mit Bettung"
    } else {
        "roadbed"
    };
    match cat.conductors {
        Some(3) => format!(
            "{gauge} mm, {roadbed}, {}",
            if is_de { "3-Leiter" } else { "3-rail" }
        ),
        Some(2) => format!(
            "{gauge} mm, {roadbed}, {}",
            if is_de { "2-Leiter" } else { "2-rail" }
        ),
        Some(n) => format!(
            "{gauge} mm, {roadbed}, {n}{}",
            if is_de { " Leiter" } else { "-conductor" }
        ),
        None => format!("{gauge} mm, {roadbed}"),
    }
}

pub(crate) fn collect_track_manufacturers_in_order(
    catalogs: &[railbench_core::types::Catalog],
    indices: &[usize],
) -> Vec<String> {
    let mut manufacturers = Vec::new();
    for &idx in indices {
        let m = catalogs[idx]
            .manufacturer
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Other".into());
        if !manufacturers.contains(&m) {
            manufacturers.push(m);
        }
    }
    manufacturers
}

pub(super) fn draw_popover_divider(ui: &mut egui::Ui, dark_mode: bool) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 5.0), Sense::hover());
    let col = if dark_mode {
        Color32::from_rgba_unmultiplied(255, 255, 255, 20)
    } else {
        Color32::from_rgba_unmultiplied(15, 23, 42, 20)
    };
    ui.painter().hline(r.x_range(), r.center().y, Stroke::new(1.0_f32, col));
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_popover_row(
    ui: &mut egui::Ui,
    p: crate::theme::RailbenchPalette,
    dark_mode: bool,
    title: &str,
    subtitle: Option<&str>,
    right_badge: Option<&str>,
    active: bool,
    title_color_override: Option<Color32>,
) -> bool {
    let h = if subtitle.is_some() { 40.0 } else { 30.0 };
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());
    let fill = if active {
        Color32::from_rgb(2, 132, 199)
    } else if resp.hovered() {
        if dark_mode {
            Color32::from_rgba_unmultiplied(255, 255, 255, 20)
        } else {
            Color32::from_rgba_unmultiplied(15, 23, 42, 14)
        }
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, fill);
    let fg = if active {
        Color32::WHITE
    } else {
        title_color_override.unwrap_or(p.text_main)
    };
    let sub_fg = if active {
        Color32::from_rgba_unmultiplied(255, 255, 255, 210)
    } else {
        p.text_muted
    };
    if let Some(badge) = right_badge {
        painter.text(
            Pos2::new(rect.max.x - 10.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            badge,
            FontId::proportional(12.0),
            if active {
                Color32::WHITE
            } else {
                title_color_override.unwrap_or(p.text_muted)
            },
        );
    }
    let text_right = if right_badge.is_some() {
        rect.max.x - 26.0
    } else {
        rect.max.x - 10.0
    };
    let clip = Rect::from_min_max(Pos2::new(rect.min.x + 10.0, rect.min.y), Pos2::new(text_right, rect.max.y));
    let tp = ui.painter_at(clip);
    if let Some(sub) = subtitle {
        tp.text(
            Pos2::new(clip.min.x, rect.min.y + 6.0),
            egui::Align2::LEFT_TOP,
            title,
            FontId::proportional(12.0),
            fg,
        );
        tp.text(
            Pos2::new(clip.min.x, rect.min.y + 22.0),
            egui::Align2::LEFT_TOP,
            sub,
            FontId::proportional(10.0),
            sub_fg,
        );
    } else {
        tp.text(
            Pos2::new(clip.min.x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            title,
            FontId::proportional(12.0),
            fg,
        );
    }
    resp.clicked()
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
    ui.label(
        RichText::new(if is_de {
            "3D-MODELLE / EIGENE & VORLAGEN"
        } else {
            "3D MODELS / CUSTOM & BUILT-IN"
        })
        .strong()
        .size(11.0)
        .color(p.text_muted),
    );
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
                    .unwrap_or(if is_de { "Spur H0 (1:87)" } else { "Scale H0 (1:87)" });
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
                let built_tpl = model_tools::create_scaled_builtin_model(
                    def,
                    &app.model_scale,
                    is_de,
                );
                let mut add_clicked = false;
                let card_resp = Frame::new()
                    .fill(p.bg_card)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(10, 7))
                    .stroke(Stroke::new(1.0_f32, p.border_main))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(name).strong().size(12.0).color(p.text_main));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button(if is_de { "+ Hinzuf." } else { "+ Add" }).clicked() {
                                    add_clicked = true;
                                }
                            });
                        });
                        ui.label(
                            RichText::new(format!("{w:.0} x {d:.0} x {h:.0} mm ({})", app.model_scale))
                                .size(10.5)
                                .color(p.primary),
                        );
                        ui.label(RichText::new(desc).size(10.0).color(p.text_muted));
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
                if add_clicked {
                    place_model = Some(built_tpl);
                } else if card_resp.drag_started_by(egui::PointerButton::Primary) {
                    app.dragging_model_template = Some(built_tpl);
                }
                ui.add_space(4.0);
            }
        }

        if cat == "all" || cat == "custom" {
            for cm in &app.custom_models_library {
                if !q.is_empty() && !cm.name.to_ascii_lowercase().contains(&q) {
                    continue;
                }
                let mut add_clicked = false;
                let mut del_clicked = false;
                let card_resp = Frame::new()
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
                                if ui.small_button(if is_de { "Entf." } else { "Del" }).clicked() {
                                    del_clicked = true;
                                }
                                if ui.small_button(if is_de { "+ Hinzuf." } else { "+ Add" }).clicked() {
                                    add_clicked = true;
                                }
                            });
                        });
                        ui.label(
                            RichText::new(format!("{:.0} x {:.0} x {:.0} mm", cm.width, cm.depth, cm.height))
                                .size(10.5)
                                .color(p.text_muted),
                        );
                    })
                    .response
                    .interact(egui::Sense::click_and_drag());
                if card_resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
                if del_clicked {
                    remove_custom_name = Some(cm.name.clone());
                } else if add_clicked {
                    place_model = Some(cm.clone());
                } else if card_resp.drag_started_by(egui::PointerButton::Primary) {
                    app.dragging_model_template = Some(cm.clone());
                }
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


/// Favorites accent color: bright amber/yellow on dark backgrounds, deep amber (#b45309)
/// in light mode so the star and labels keep strong contrast on crema surfaces.
pub(crate) fn favorite_accent(dark_mode: bool, dark_col: Color32) -> Color32 {
    if dark_mode {
        dark_col
    } else {
        Color32::from_rgb(180, 83, 9)
    }
}
