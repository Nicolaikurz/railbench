use std::collections::BTreeMap;

use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Stroke};
use railbench_core::types::{BaseboardConfig, Elevation, Layer};

use crate::canvas2d::{parse_hex_color, ColorMode};
use crate::i18n::{tr, Lang};
use crate::icons::{icon_button, icon_button_sized, Icon};
use crate::theme::{palette, style_toolbar_ui};
use crate::{RailbenchApp, SidebarTab};

pub fn draw_top_bars(ctx: &egui::Context, app: &mut RailbenchApp) {
    let t = tr(app.lang);
    let p = palette(app.dark_mode);

    // =========================================================================
    //  LINE 1: Menubar Row (Brand + Menus on left; Filename + Track count right)
    //  Matches src/ui/toolbar.ts toolbar-menubar-row
    // =========================================================================
    egui::TopBottomPanel::top("menu_bar")
        .frame(
            Frame::new()
                .fill(p.bg_toolbar)
                .inner_margin(Margin::symmetric(12, 5))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 30))),
        )
        .show(ctx, |ui| {
            style_toolbar_ui(ui, p);
            egui::menu::bar(ui, |ui| {
                // RAILbench brand badge
                ui.label(RichText::new("RAIL").strong().size(14.5).color(p.primary));
                ui.add_space(-6.0);
                ui.label(RichText::new("bench").strong().size(14.5).color(Color32::WHITE));
                ui.add_space(10.0);

                ui.menu_button(t.menu_file, |ui| {
                    if ui.button(t.new_layout).clicked() {
                        app.new_empty_layout();
                        ui.close_menu();
                    }
                    if ui.button(t.open_file).clicked() {
                        app.open_layout_dialog();
                        ui.close_menu();
                    }
                    ui.menu_button(t.open_demo, |ui| {
                        for (label, path) in [
                            ("Alpenland H0 Demo", "demotracks/alpenland-h0.railbench"),
                            ("Mainline H0", "demotracks/mainline-h0.railbench"),
                            ("Shunting H0", "demotracks/shunting-h0.railbench"),
                            ("Switching N", "demotracks/switching-n.railbench"),
                            ("Garden G", "demotracks/garden-g.railbench"),
                            ("Suitcase Z", "demotracks/suitcase-z.railbench"),
                        ] {
                            if ui.button(label).clicked() {
                                app.open_layout_path(path);
                                ui.close_menu();
                            }
                        }
                    });
                    if ui.button(t.save_file).clicked() {
                        app.save_layout_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Export PDF Blueprint...").clicked() {
                        app.export_pdf_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Export SVG Blueprint...").clicked() {
                        app.export_svg_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Export BOM (CSV)...").clicked() {
                        app.export_bom_csv_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Collaborate (WebSocket)...").clicked() {
                        app.show_collab_dialog = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button(t.menu_edit, |ui| {
                    if ui.add_enabled(!app.undo_stack.is_empty(), egui::Button::new(t.undo)).clicked() {
                        app.undo();
                        ui.close_menu();
                    }
                    if ui.add_enabled(!app.redo_stack.is_empty(), egui::Button::new(t.redo)).clicked() {
                        app.redo();
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
                    if ui.button(t.delete_sel).clicked() {
                        app.delete_selected();
                        ui.close_menu();
                    }
                });

                ui.menu_button(t.menu_view, |ui| {
                    if ui.button(t.reset_view).clicked() {
                        app.pending_fit_view = true;
                        ui.close_menu();
                    }
                    ui.checkbox(&mut app.show_labels, t.labels);
                    ui.checkbox(&mut app.show_connectors, t.connectors);
                    ui.checkbox(&mut app.show_rulers, "Rulers");
                    ui.checkbox(&mut app.power_sim_enabled, t.volt_btn);
                    ui.separator();
                    ui.radio_value(&mut app.color_mode, ColorMode::Types, t.color_types);
                    ui.radio_value(&mut app.color_mode, ColorMode::Layers, t.color_layers);
                    ui.radio_value(&mut app.color_mode, ColorMode::Off, t.color_off);
                });

                ui.menu_button(t.menu_settings, |ui| {
                    if ui.button(t.power_dialog).clicked() {
                        app.show_power_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button(t.height_dialog).clicked() {
                        app.show_height_dialog = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button(t.menu_help, |ui| {
                    if ui.button("Shortcuts & Info").clicked() {
                        app.show_help_dialog = true;
                        ui.close_menu();
                    }
                });

                // Right side of Line 1: Window Controls (Close / Max / Min) + Track Count + Filename
                let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let win_sz = egui::vec2(28.0, 22.0);
                    // Close button (red hover)
                    let close_resp = icon_button_sized(
                        ui,
                        Icon::WinClose,
                        win_sz,
                        false,
                        Color32::TRANSPARENT,
                        Color32::from_rgb(220, 38, 38),
                        Color32::from_rgb(239, 68, 68),
                        Color32::from_rgb(226, 232, 240),
                        "Close",
                    );
                    if close_resp.hovered() {
                        ui.painter().rect_filled(close_resp.rect, CornerRadius::same(6), Color32::from_rgb(220, 38, 38));
                        crate::icons::paint_icon(
                            ui.painter(),
                            close_resp.rect.shrink2(egui::vec2(7.0, 4.0)),
                            Icon::WinClose,
                            Color32::WHITE,
                        );
                    }
                    if close_resp.clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    // Maximize / Restore button
                    let max_icon = if is_maximized { Icon::WinRestore } else { Icon::WinMaximize };
                    let max_tip = if is_maximized { "Restore" } else { "Maximize" };
                    if icon_button_sized(
                        ui,
                        max_icon,
                        win_sz,
                        false,
                        Color32::TRANSPARENT,
                        p.toolbar_btn_hover,
                        p.primary,
                        Color32::from_rgb(226, 232, 240),
                        max_tip,
                    )
                    .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                    }

                    // Minimize button
                    if icon_button_sized(
                        ui,
                        Icon::WinMinimize,
                        win_sz,
                        false,
                        Color32::TRANSPARENT,
                        p.toolbar_btn_hover,
                        p.primary,
                        Color32::from_rgb(226, 232, 240),
                        "Minimize",
                    )
                    .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }

                    ui.add_space(8.0);
                    Frame::new()
                        .fill(p.toolbar_btn)
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("{} {}", app.layout.pieces.len(), t.status_pieces))
                                    .size(11.5)
                                    .strong()
                                    .color(p.primary),
                            );
                        });
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!("{}.railbench", app.layout.name))
                            .size(12.0)
                            .color(Color32::from_rgb(203, 213, 225)),
                    );

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

    // =========================================================================
    //  LINE 2: Actions Row (Sidebar Register Tabs left | Tools center | DE/Collab/Theme/Layers right)
    //  Matches src/ui/toolbar.ts toolbar-actions-row
    // =========================================================================
    egui::TopBottomPanel::top("quick_toolbar")
        .frame(
            Frame::new()
                .fill(p.bg_toolbar)
                .inner_margin(Margin::symmetric(10, 5))
                .stroke(Stroke::new(1.0_f32, p.border_main)),
        )
        .show(ctx, |ui| {
            style_toolbar_ui(ui, p);
            ui.horizontal(|ui| {
                let pill_btn = |ui: &mut egui::Ui, label: &str, active: bool| -> egui::Response {
                    let fill = if active { p.primary_hover } else { p.toolbar_btn };
                    let stroke = if active {
                        Stroke::new(1.2_f32, p.primary)
                    } else {
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 35))
                    };
                    ui.add(
                        egui::Button::new(RichText::new(label).size(12.0).color(Color32::WHITE))
                            .fill(fill)
                            .stroke(stroke)
                            .corner_radius(CornerRadius::same(6)),
                    )
                };

                // Sidebar Register Tabs (aligned above left sidebar, matching #toolbar-sidebar-tabs)
                ui.spacing_mut().item_spacing.x = 3.0;
                for (tab, lbl) in [
                    (SidebarTab::Tracks, t.tab_tracks),
                    (SidebarTab::Roads, t.tab_roads),
                    (SidebarTab::Models, t.tab_models),
                    (SidebarTab::Terrain, t.tab_terrain),
                    (SidebarTab::Board, t.tab_board),
                    (SidebarTab::Bom, t.tab_bom),
                ] {
                    let active = app.sidebar_tab == tab;
                    let fill = if active { p.primary_hover } else { Color32::TRANSPARENT };
                    let txt_col = if active { Color32::WHITE } else { Color32::from_rgb(148, 163, 184) };
                    let resp = ui.add(
                        egui::Button::new(RichText::new(lbl).strong().size(11.5).color(txt_col))
                            .fill(fill)
                            .stroke(if active {
                                Stroke::new(1.0_f32, p.primary)
                            } else {
                                Stroke::NONE
                            })
                            .corner_radius(CornerRadius::same(5)),
                    );
                    if resp.clicked() {
                        app.sidebar_tab = tab;
                    }
                }
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.separator();

                let tb_icon = |ui: &mut egui::Ui, icon: Icon, active: bool, tip: &str| -> bool {
                    icon_button(ui, icon, active, p.toolbar_btn, p.primary_hover, p.primary, Color32::WHITE, tip).clicked()
                };

                // Quick file actions (monochrome icons)
                if tb_icon(ui, Icon::FilePlus, false, t.new_layout) { app.new_empty_layout(); }
                if tb_icon(ui, Icon::FolderOpen, false, t.open_file) { app.open_layout_dialog(); }
                if tb_icon(ui, Icon::Save, false, t.save_file) { app.save_layout_dialog(); }
                ui.separator();

                // Undo / Redo (monochrome icons)
                ui.add_enabled_ui(!app.undo_stack.is_empty(), |ui| {
                    if tb_icon(ui, Icon::Undo, false, t.undo) {
                        app.undo();
                    }
                });
                ui.add_enabled_ui(!app.redo_stack.is_empty(), |ui| {
                    if tb_icon(ui, Icon::Redo, false, t.redo) {
                        app.redo();
                    }
                });
                ui.separator();

                // Snap + Measure + Rotate split group (monochrome icons)
                if tb_icon(ui, Icon::Magnet, app.snapping_enabled, t.snapping) {
                    app.snapping_enabled = !app.snapping_enabled;
                }
                if tb_icon(ui, Icon::Ruler, app.measure_mode, "Measure distance") {
                    app.measure_mode = !app.measure_mode;
                    if !app.measure_mode {
                        app.measure_points = None;
                    }
                }
                if tb_icon(ui, Icon::RotateCcw, false, t.rotate_ccw) {
                    app.rotate_selected(-app.rotation_step);
                }
                if tb_icon(ui, Icon::RotateCw, false, t.rotate_cw) {
                    app.rotate_selected(app.rotation_step);
                }
                egui::ComboBox::from_id_salt("rot_step")
                    .width(54.0)
                    .selected_text(format!("{}°", app.rotation_step))
                    .show_ui(ui, |ui| {
                        for deg in [5.7, 7.5, 15.0, 24.3, 30.0, 45.0, 90.0] {
                            ui.selectable_value(&mut app.rotation_step, deg, format!("{deg}°"));
                        }
                    });

                // Colors + Labels + Voltage + 3D (monochrome icons)
                let colors_active = app.color_mode != ColorMode::Off;
                if tb_icon(ui, Icon::Palette, colors_active, t.colors_btn) {
                    app.color_mode = match app.color_mode {
                        ColorMode::Types => ColorMode::Layers,
                        ColorMode::Layers => ColorMode::Off,
                        ColorMode::Off => ColorMode::Types,
                    };
                }
                if tb_icon(ui, Icon::Tag, app.show_labels, t.labels) {
                    app.show_labels = !app.show_labels;
                }

                // Voltage / Power Sim toggle + presets menu
                if tb_icon(ui, Icon::Zap, app.power_sim_enabled, t.volt_btn) {
                    app.power_sim_enabled = !app.power_sim_enabled;
                }
                ui.menu_button("▾", |ui| {
                    for (lbl, v, ma) in [
                        ("10V Spur Z (100 mA)", 10.0, 100.0),
                        ("12V Spur N (100 mA)", 12.0, 100.0),
                        ("14V Spur TT (300 mA)", 14.0, 300.0),
                        ("14V H0 DCC (450 mA)", 14.0, 450.0),
                        ("16V H0 Standard (450 mA)", 16.0, 450.0),
                        ("18V Spur 1 (800 mA)", 18.0, 800.0),
                        ("24V Spur G / LGB (1000 mA)", 24.0, 1000.0),
                    ] {
                        if ui.button(lbl).clicked() {
                            app.layout.feed_voltage = Some(v);
                            app.layout.loco_current_ma = Some(ma);
                            app.power_sim_enabled = true;
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button(t.power_dialog).clicked() {
                        app.show_power_dialog = true;
                        ui.close_menu();
                    }
                });

                // 3D Toggle
                if tb_icon(ui, Icon::Box3d, app.is_3d_mode, "3D View (3)") {
                    app.toggle_3d();
                }

                // Right-aligned utilities (matches TS toolbar-right: DE/EN, Collab, Theme, Layers)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if tb_icon(ui, Icon::Layers, app.show_right_sidebar, t.layers_title) {
                        app.show_right_sidebar = !app.show_right_sidebar;
                    }
                    let theme_icon = if app.dark_mode { Icon::Moon } else { Icon::Sun };
                    let theme_tip = if app.dark_mode { t.theme_dark } else { t.theme_light };
                    if tb_icon(ui, theme_icon, false, theme_tip) {
                        app.dark_mode = !app.dark_mode;
                    }
                    if tb_icon(ui, Icon::Users, app.collab.connected, "Live Collaboration") {
                        app.show_collab_dialog = !app.show_collab_dialog;
                    }
                    if pill_btn(ui, if app.lang == Lang::De { "DE" } else { "EN" }, false).clicked() {
                        app.lang = if app.lang == Lang::De { Lang::En } else { Lang::De };
                    }
                });
            });
        });
}

pub fn draw_left_sidebar(ctx: &egui::Context, app: &mut RailbenchApp) {
    let t = tr(app.lang);
    let p = palette(app.dark_mode);

    egui::SidePanel::left("left_sidebar")
        .default_width(276.0)
        .min_width(220.0)
        .resizable(true)
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
            let _ = (t, p);
        });
}

fn draw_catalog_tab(ui: &mut egui::Ui, app: &mut RailbenchApp, roads_only: bool) {
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

    if roads_only {
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

        ui.label(RichText::new(mfr_label).strong().size(10.0).color(p.text_muted));
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
    ui.add(
        egui::TextEdit::singleline(&mut app.catalog_search)
            .hint_text(t.search_hint)
            .desired_width(f32::INFINITY),
    );
    ui.add_space(8.0);

    let filter = app.catalog_search.trim().to_lowercase();
    let entries = app.catalogs[*sel_idx].entries.clone();

    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut last_cat = String::new();
        for entry in &entries {
            let display_name = if app.lang == Lang::En {
                entry.name_en.as_deref().unwrap_or(&entry.name)
            } else {
                &entry.name
            };
            if !filter.is_empty()
                && !entry.id.to_lowercase().contains(&filter)
                && !display_name.to_lowercase().contains(&filter)
                && !entry.category.to_lowercase().contains(&filter)
            {
                continue;
            }
            if entry.category != last_cat {
                if !last_cat.is_empty() {
                    ui.add_space(6.0);
                }
                ui.label(
                    RichText::new(entry.category.to_uppercase())
                        .strong()
                        .size(10.5)
                        .color(p.text_muted),
                );
                last_cat = entry.category.clone();
            }

            let accent = match entry.kind.as_str() {
                "curve" => Color32::from_rgb(59, 130, 246),
                "turnout-left" | "turnout-right" | "three-way" | "double-slip" | "curved-turnout-left" | "curved-turnout-right" => {
                    Color32::from_rgb(245, 158, 11)
                }
                "flex" => Color32::from_rgb(6, 182, 212),
                "crossing" => Color32::from_rgb(168, 85, 247),
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

            let card_resp = Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(9, 6))
                .stroke(Stroke::new(1.0_f32, p.border_main))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                        draw_track_kind_icon(ui.painter(), icon_rect, &entry.kind, accent, p.bg_input);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&entry.id).strong().size(12.5).color(p.text_main));
                                if !dim_text.is_empty() {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(RichText::new(&dim_text).size(10.5).color(p.text_muted));
                                    });
                                }
                            });
                            ui.label(RichText::new(display_name).size(11.0).color(p.text_sub));
                        });
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
            if card_resp.drag_started_by(egui::PointerButton::Primary) {
                app.dragging_catalog_entry = Some(entry.clone());
            }
            if card_resp.clicked() {
                app.place_catalog_entry(entry);
            }
        }
    });
}

fn draw_models_tab(ui: &mut egui::Ui, app: &mut RailbenchApp) {
    let p = palette(app.dark_mode);
    ui.label(RichText::new("3D-MODELLE / BUILDINGS").strong().size(11.0).color(p.text_muted));
    ui.add_space(6.0);

    let items = [
        ("station", "Bahnhof / Station", 320.0, 140.0, 130.0),
        ("town-house", "Stadthaus / Town House", 120.0, 100.0, 140.0),
        ("signal-box", "Stellwerk / Signal Box", 95.0, 75.0, 110.0),
        ("water-tower", "Wasserturm / Water Tower", 80.0, 80.0, 170.0),
        ("deciduous-tree", "Laubbaum / Deciduous Tree", 65.0, 65.0, 115.0),
        ("pine-tree", "Nadelbaum / Pine Tree", 55.0, 55.0, 130.0),
        ("light-signal", "Lichtsignal / Light Signal", 20.0, 20.0, 70.0),
        ("platform-lamp", "Bahnsteiglampe / Lamp", 18.0, 18.0, 65.0),
    ];

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (bid, name, w, d, h) in items {
            let resp = Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(10, 7))
                .stroke(Stroke::new(1.0_f32, p.border_main))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(name).strong().size(12.5).color(p.text_main));
                    ui.label(RichText::new(format!("{w:.0} × {d:.0} × {h:.0} mm")).size(10.5).color(p.text_muted));
                })
                .response
                .interact(egui::Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                app.place_builtin_model(bid, name, w, d, h);
            }
        }
    });
}

fn draw_terrain_tab(ui: &mut egui::Ui, app: &mut RailbenchApp) {
    let p = palette(app.dark_mode);
    ui.label(RichText::new("GELÄNDE / TERRAIN").strong().size(11.0).color(p.text_muted));
    ui.add_space(6.0);

    for (name, w, d, h) in [
        ("Alpenberg / Alpine Mountain", 650.0, 520.0, 260.0),
        ("Hügel / Rolling Hill", 480.0, 380.0, 140.0),
        ("Felsmassiv / Rock Ridge", 800.0, 420.0, 320.0),
    ] {
        let resp = Frame::new()
            .fill(p.bg_card)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(10, 7))
            .stroke(Stroke::new(1.0_f32, p.border_main))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(name).strong().size(12.5).color(p.text_main));
                ui.label(RichText::new(format!("{w:.0} × {d:.0} mm  •  H: {h:.0} mm")).size(10.5).color(p.text_muted));
            })
            .response
            .interact(egui::Sense::click());
        if resp.clicked() {
            app.place_builtin_model("mountain", name, w, d, h);
        }
    }
}

fn draw_board_tab(ui: &mut egui::Ui, app: &mut RailbenchApp) {
    let p = palette(app.dark_mode);
    ui.label(RichText::new("GRUNDPLATTE / BASEBOARD").strong().size(11.0).color(p.text_muted));
    ui.add_space(6.0);

    let board = app.layout.baseboard.get_or_insert_with(|| BaseboardConfig {
        id: Some("board-1".into()),
        name: Some("Tisch".into()),
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 2400.0,
        depth: 1200.0,
        rotation: Some(0.0),
        l_wing_width: None,
        l_wing_depth: None,
        l_side: None,
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        position_mode: "center".into(),
        custom_x: Some(0.0),
        custom_y: Some(0.0),
        style: "wood".into(),
        corner_radius: Some(12.0),
        show_dimensions: Some(true),
    });

    let mut changed = false;
    Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .stroke(Stroke::new(1.0_f32, p.border_main))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            changed |= ui.checkbox(&mut board.enabled, "Baseboard visible").changed();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Width:");
                changed |= ui
                    .add(egui::DragValue::new(&mut board.width).range(400.0..=12000.0).speed(20.0).suffix(" mm"))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Depth:");
                changed |= ui
                    .add(egui::DragValue::new(&mut board.depth).range(300.0..=8000.0).speed(20.0).suffix(" mm"))
                    .changed();
            });
            ui.add_space(6.0);
            ui.label(RichText::new("Presets").size(11.0).color(p.text_muted));
            ui.horizontal_wrapped(|ui| {
                for (lbl, w, d) in [
                    ("2000×1000", 2000.0, 1000.0),
                    ("2400×1200", 2400.0, 1200.0),
                    ("3000×1500", 3000.0, 1500.0),
                ] {
                    if ui.button(lbl).clicked() {
                        board.width = w;
                        board.depth = d;
                        board.enabled = true;
                        changed = true;
                    }
                }
            });
        });

    if changed {
        app.mark_layout_dirty();
    }
}

fn draw_bom_tab(ui: &mut egui::Ui, app: &mut RailbenchApp) {
    let t = tr(app.lang);
    let p = palette(app.dark_mode);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} ({})", t.tab_bom.to_uppercase(), app.layout.pieces.len()))
                .strong()
                .size(11.0)
                .color(p.text_muted),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new(t.export_csv).size(11.0).color(Color32::WHITE))
                        .fill(p.primary_hover)
                        .corner_radius(CornerRadius::same(6)),
                )
                .clicked()
            {
                app.export_bom_csv_dialog();
            }
        });
    });
    ui.add_space(6.0);

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for piece in &app.layout.pieces {
        *counts.entry(piece.catalog_id.as_str()).or_insert(0) += 1;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (cid, cnt) in counts {
            let name = crate::catalog::find_entry(&app.catalogs, cid)
                .map(|e| {
                    if app.lang == Lang::En {
                        e.name_en.as_deref().unwrap_or(&e.name)
                    } else {
                        &e.name
                    }
                })
                .unwrap_or("");
            Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(7))
                .inner_margin(Margin::symmetric(9, 6))
                .stroke(Stroke::new(1.0_f32, p.border_main))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{cnt}×")).strong().color(p.primary));
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(RichText::new(cid).strong().size(12.5).color(p.text_main));
                            if !name.is_empty() {
                                ui.label(RichText::new(name).size(10.5).color(p.text_muted));
                            }
                        });
                    });
                });
        }
    });
}

pub fn draw_right_sidebar(ctx: &egui::Context, app: &mut RailbenchApp) {
    if !app.show_right_sidebar {
        return;
    }
    let t = tr(app.lang);
    let p = palette(app.dark_mode);

    egui::SidePanel::right("right_sidebar")
        .default_width(248.0)
        .min_width(205.0)
        .resizable(true)
        .frame(
            Frame::new()
                .fill(p.bg_sidebar)
                .inner_margin(Margin::same(10))
                .stroke(Stroke::new(1.0_f32, p.border_main)),
        )
        .show(ctx, |ui| {
            // Header
            ui.label(RichText::new(t.layers_title.to_uppercase()).strong().size(11.5).color(p.text_muted));
            ui.add_space(6.0);

            // Count pieces per layer
            let mut layer_counts: BTreeMap<String, usize> = BTreeMap::new();
            for piece in &app.layout.pieces {
                let lid = piece.layer_id.clone().unwrap_or_else(|| "layer-1".into());
                *layer_counts.entry(lid).or_insert(0) += 1;
            }

            let active_id = app.layout.active_layer_id.clone().unwrap_or_else(|| "layer-1".into());
            let mut delete_layer_id: Option<String> = None;
            let total_layers = app.layout.layers.as_ref().map(|l| l.len()).unwrap_or(0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(layers) = &mut app.layout.layers {
                    for layer in layers.iter_mut() {
                        let is_act = layer.id == active_id;
                        let swatch = parse_hex_color(layer.color.as_deref()).unwrap_or(p.primary);
                        let stroke = if is_act {
                            Stroke::new(1.4_f32, p.primary)
                        } else {
                            Stroke::new(1.0_f32, p.border_main)
                        };
                        let cnt = layer_counts.get(&layer.id).copied().unwrap_or(0);

                        Frame::new()
                            .fill(p.bg_card)
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::symmetric(8, 6))
                            .stroke(stroke)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let (bar_r, _) = ui.allocate_exact_size(egui::vec2(4.0, 26.0), egui::Sense::hover());
                                    ui.painter().rect_filled(bar_r, 2.0, swatch);

                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 2.0;
                                        ui.horizontal(|ui| {
                                            if ui
                                                .selectable_label(is_act, RichText::new(&layer.name).strong().size(12.0))
                                                .clicked()
                                            {
                                                app.layout.active_layer_id = Some(layer.id.clone());
                                            }
                                            if is_act {
                                                Frame::new()
                                                    .fill(p.primary_hover)
                                                    .corner_radius(CornerRadius::same(3))
                                                    .inner_margin(Margin::symmetric(4, 1))
                                                    .show(ui, |ui| {
                                                        ui.label(
                                                            RichText::new(t.active_badge)
                                                                .strong()
                                                                .size(8.5)
                                                                .color(Color32::WHITE),
                                                        );
                                                    });
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{:.0} mm  •  {} {}",
                                                    layer.height_mm.unwrap_or(0.0),
                                                    cnt,
                                                    t.status_pieces
                                                ))
                                                .size(10.0)
                                                .color(p.text_muted),
                                            );
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let sz = egui::vec2(22.0, 20.0);
                                                if total_layers > 1
                                                    && icon_button_sized(
                                                        ui,
                                                        Icon::Trash,
                                                        sz,
                                                        false,
                                                        p.bg_input,
                                                        Color32::from_rgb(220, 38, 38),
                                                        Color32::from_rgb(220, 38, 38),
                                                        p.text_sub,
                                                        "Delete layer",
                                                    )
                                                    .clicked()
                                                {
                                                    delete_layer_id = Some(layer.id.clone());
                                                }
                                                let lock_icon = if layer.locked { Icon::Lock } else { Icon::Unlock };
                                                if icon_button_sized(
                                                    ui,
                                                    lock_icon,
                                                    sz,
                                                    layer.locked,
                                                    p.bg_input,
                                                    p.primary_hover,
                                                    p.primary,
                                                    p.text_sub,
                                                    "Lock layer",
                                                )
                                                .clicked()
                                                {
                                                    layer.locked = !layer.locked;
                                                }
                                                let vis_icon = if layer.visible { Icon::Eye } else { Icon::EyeOff };
                                                if icon_button_sized(
                                                    ui,
                                                    vis_icon,
                                                    sz,
                                                    false,
                                                    p.bg_input,
                                                    p.primary_hover,
                                                    p.primary,
                                                    if layer.visible { p.text_main } else { p.text_muted },
                                                    "Toggle visibility",
                                                )
                                                .clicked()
                                                {
                                                    layer.visible = !layer.visible;
                                                }
                                            });
                                        });
                                    });
                                });
                            });
                    }
                }

                ui.add_space(6.0);
                if ui
                    .add_sized(
                        [ui.available_width(), 28.0],
                        egui::Button::new(RichText::new(t.add_layer_full).strong().size(12.0).color(p.text_main))
                            .fill(p.bg_card)
                            .stroke(Stroke::new(1.0_f32, p.border_main))
                            .corner_radius(CornerRadius::same(7)),
                    )
                    .clicked()
                {
                    app.add_layer();
                }

                ui.add_space(6.0);
                if ui
                    .add_sized(
                        [ui.available_width(), 28.0],
                        egui::Button::new(RichText::new(t.heights_ramp_btn).strong().size(12.0).color(Color32::WHITE))
                            .fill(p.primary_hover)
                            .corner_radius(CornerRadius::same(7)),
                    )
                    .clicked()
                {
                    app.show_height_dialog = true;
                }
            });

            if let Some(del_id) = delete_layer_id {
                app.push_undo();
                if let Some(layers) = &mut app.layout.layers {
                    layers.retain(|l| l.id != del_id);
                    if app.layout.active_layer_id.as_deref() == Some(del_id.as_str()) {
                        app.layout.active_layer_id = layers.first().map(|l| l.id.clone());
                    }
                }
            }
        });
}

/// Floating pill bar at bottom-center of canvas when 1+ tracks are selected (matches src/ui/selection-bar.ts).
pub fn draw_floating_selection_bar(ctx: &egui::Context, app: &mut RailbenchApp) {
    let t = tr(app.lang);
    let p = palette(app.dark_mode);

    // Subtle bottom-right zoom indicator pill (always visible, unobtrusive)
    egui::Area::new(egui::Id::new("zoom_hud_pill"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -14.0))
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(p.bg_toolbar.r(), p.bg_toolbar.g(), p.bg_toolbar.b(), 215))
                .corner_radius(CornerRadius::same(14))
                .inner_margin(Margin::symmetric(10, 4))
                .stroke(Stroke::new(1.0_f32, p.border_main))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(msg) = &app.status_message {
                            ui.label(RichText::new(msg).size(11.0).color(Color32::from_rgb(148, 163, 184)));
                            ui.label(RichText::new("•").size(11.0).color(Color32::from_rgb(100, 116, 139)));
                        }
                        ui.label(
                            RichText::new(format!("{:.0}%", app.camera.zoom * 100.0))
                                .strong()
                                .size(11.0)
                                .color(Color32::WHITE),
                        );
                    });
                });
        });

    if app.selected.is_empty() {
        return;
    }

    let sel_count = app.selected.len();
    let first_sel = app
        .layout
        .pieces
        .iter()
        .find(|pc| app.selected.contains(&pc.id))
        .cloned();

    let Some(sample) = first_sel else {
        // Selected item(s) are 3D buildings/models: show compact model selection bar
        let first_model = app
            .layout
            .models
            .as_ref()
            .and_then(|ms| ms.iter().find(|m| app.selected.contains(&m.id)))
            .cloned();
        let Some(model) = first_model else { return };
        egui::Area::new(egui::Id::new("floating_selection_bar"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -14.0))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(p.bg_card.r(), p.bg_card.g(), p.bg_card.b(), 242))
                    .corner_radius(CornerRadius::same(20))
                    .inner_margin(Margin::symmetric(12, 6))
                    .stroke(Stroke::new(1.2_f32, p.border_main))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let lbl = if sel_count == 1 {
                                model.name.clone()
                            } else {
                                format!("{sel_count}×")
                            };
                            ui.label(RichText::new(lbl).strong().size(12.0).color(p.primary));
                            ui.separator();
                            let sz = egui::vec2(26.0, 24.0);
                            if icon_button_sized(ui, Icon::RotateCcw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_ccw).clicked() {
                                app.rotate_selected(-app.rotation_step);
                            }
                            if icon_button_sized(ui, Icon::RotateCw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_cw).clicked() {
                                app.rotate_selected(app.rotation_step);
                            }
                            if icon_button_sized(
                                ui,
                                Icon::Trash,
                                sz,
                                true,
                                p.bg_input,
                                Color32::from_rgb(220, 38, 38),
                                Color32::from_rgb(239, 68, 68),
                                Color32::WHITE,
                                t.delete_sel,
                            )
                            .clicked()
                            {
                                app.delete_selected();
                            }
                        });
                    });
            });
        return;
    };

    let has_feeder = sample.power_feed == Some(true);
    let has_iso = sample.is_isolator == Some(true);
    let has_cat = sample.has_catenary == Some(true);
    let has_bridge = sample.has_bridge == Some(true);
    let has_tunnel = sample.has_tunnel == Some(true);
    let mut z_mm = sample.elevation.map(|e| e.start).unwrap_or(0.0);

    egui::Area::new(egui::Id::new("floating_selection_bar"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -14.0))
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(p.bg_card.r(), p.bg_card.g(), p.bg_card.b(), 242))
                .corner_radius(CornerRadius::same(20))
                .inner_margin(Margin::symmetric(12, 6))
                .stroke(Stroke::new(1.2_f32, p.border_main))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 6],
                    blur: 18,
                    spread: 0,
                    color: Color32::from_black_alpha(80),
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Selection count + ID
                        let badge_txt = if sel_count == 1 {
                            sample.catalog_id.clone()
                        } else {
                            format!("{sel_count}×")
                        };
                        ui.label(RichText::new(badge_txt).strong().size(12.0).color(p.primary));
                        ui.separator();

                        // Flextrack / Flex-street Length & Bend Angle controls
                        if sel_count == 1 && crate::catalog::is_flex_id(&app.catalogs, &sample.catalog_id) {
                            let default_len = crate::catalog::find_entry(&app.catalogs, &sample.catalog_id)
                                .and_then(|e| e.length)
                                .unwrap_or(900.0);
                            let mut flex_len = sample.custom_length.unwrap_or(default_len);
                            let mut flex_ang = sample.custom_angle.unwrap_or(0.0);
                            let mut flex_changed = false;

                            ui.label(RichText::new("L:").size(11.5).color(p.text_muted));
                            flex_changed |= ui
                                .add(
                                    egui::DragValue::new(&mut flex_len)
                                        .range(25.0..=default_len.max(1200.0))
                                        .speed(2.0)
                                        .suffix(" mm"),
                                )
                                .changed();

                            ui.label(RichText::new("∠:").size(11.5).color(p.text_muted));
                            flex_changed |= ui
                                .add(
                                    egui::DragValue::new(&mut flex_ang)
                                        .range(-180.0..=180.0)
                                        .speed(1.0)
                                        .suffix("°"),
                                )
                                .changed();

                            if ui.small_button("Reset").on_hover_text("Reset flex to straight").clicked() {
                                flex_len = default_len;
                                flex_ang = 0.0;
                                flex_changed = true;
                            }

                            if flex_changed {
                                if let Some(pc) = app.layout.pieces.iter_mut().find(|x| x.id == sample.id) {
                                    crate::catalog::apply_flex_geometry(pc, &app.catalogs, flex_len, flex_ang);
                                }
                                railbench_core::snapping::update_layout_connections(&mut app.layout, 4.0, 35.0, 1.0);
                                crate::catalog::disconnect_cross_type_connections(&mut app.layout, &app.catalogs);
                                app.mark_layout_dirty();
                            }
                            ui.separator();
                        }

                        // Quick Z elevation
                        ui.label(RichText::new("Z:").size(11.5).color(p.text_muted));
                        if ui
                            .add(egui::DragValue::new(&mut z_mm).speed(1.0).suffix(" mm"))
                            .changed()
                        {
                            for pc in &mut app.layout.pieces {
                                if app.selected.contains(&pc.id) {
                                    pc.elevation = Some(Elevation { start: z_mm, end: z_mm });
                                }
                            }
                            app.mark_layout_dirty();
                        }
                        ui.separator();

                        let toggle_pill = |ui: &mut egui::Ui, label: &str, active: bool, active_col: Color32| -> bool {
                            let fill = if active { active_col } else { p.bg_input };
                            let txt = if active { Color32::WHITE } else { p.text_sub };
                            ui.add(
                                egui::Button::new(RichText::new(label).strong().size(11.5).color(txt))
                                    .fill(fill)
                                    .stroke(Stroke::new(1.0_f32, if active { active_col } else { p.border_main }))
                                    .corner_radius(CornerRadius::same(10)),
                            )
                            .clicked()
                        };

                        let sz = egui::vec2(26.0, 24.0);
                        if icon_button_sized(
                            ui,
                            Icon::Zap,
                            sz,
                            has_feeder,
                            p.bg_input,
                            Color32::from_rgb(245, 158, 11),
                            Color32::from_rgb(245, 158, 11),
                            p.text_sub,
                            t.feeder,
                        )
                        .clicked()
                        {
                            app.toggle_selected_feeder();
                        }
                        if icon_button_sized(
                            ui,
                            Icon::Isolator,
                            sz,
                            has_iso,
                            p.bg_input,
                            Color32::from_rgb(220, 38, 38),
                            Color32::from_rgb(220, 38, 38),
                            p.text_sub,
                            t.isolator,
                        )
                        .clicked()
                        {
                            app.toggle_selected_isolator();
                        }
                        if toggle_pill(ui, t.catenary, has_cat, p.primary_hover) {
                            app.toggle_selected_catenary();
                        }
                        if toggle_pill(ui, t.bridge, has_bridge, p.primary_hover) {
                            app.toggle_selected_bridge();
                        }
                        if toggle_pill(ui, t.tunnel, has_tunnel, p.primary_hover) {
                            app.toggle_selected_tunnel();
                        }

                        ui.separator();
                        if icon_button_sized(ui, Icon::RotateCcw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_ccw).clicked() {
                            app.rotate_selected(-app.rotation_step);
                        }
                        if icon_button_sized(ui, Icon::RotateCw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_cw).clicked() {
                            app.rotate_selected(app.rotation_step);
                        }
                        if icon_button_sized(
                            ui,
                            Icon::Trash,
                            sz,
                            true,
                            p.bg_input,
                            Color32::from_rgb(220, 38, 38),
                            Color32::from_rgb(239, 68, 68),
                            Color32::WHITE,
                            t.delete_sel,
                        )
                        .clicked()
                        {
                            app.delete_selected();
                        }
                    });
                });
        });
}

pub fn draw_dialogs(ctx: &egui::Context, app: &mut RailbenchApp) {
    let t = tr(app.lang);

    if app.show_power_dialog {
        egui::Window::new(t.power_dialog)
            .open(&mut app.show_power_dialog)
            .resizable(false)
            .show(ctx, |ui| {
                let mut v = app.layout.feed_voltage.unwrap_or(16.0);
                let mut ma = app.layout.loco_current_ma.unwrap_or(450.0);
                let mut cnt = app.layout.loco_count.unwrap_or(1);
                ui.horizontal(|ui| {
                    ui.label("Feed Voltage:");
                    if ui.add(egui::DragValue::new(&mut v).range(6.0..=24.0).speed(0.5).suffix(" V")).changed() {
                        app.layout.feed_voltage = Some(v);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Loco Current:");
                    if ui.add(egui::DragValue::new(&mut ma).range(50.0..=1000.0).speed(10.0).suffix(" mA")).changed() {
                        app.layout.loco_current_ma = Some(ma);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Locos:");
                    if ui.add(egui::DragValue::new(&mut cnt).range(1..=12)).changed() {
                        app.layout.loco_count = Some(cnt);
                    }
                });
                ui.separator();
                let feeders = app.layout.pieces.iter().filter(|p| p.power_feed == Some(true)).count();
                ui.label(format!("Power feeders placed: {feeders}"));
            });
    }

    if app.show_height_dialog {
        let mut open = app.show_height_dialog;
        egui::Window::new(t.height_dialog)
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Elevation:");
                    ui.add(egui::DragValue::new(&mut app.batch_elevation_mm).speed(2.0).suffix(" mm"));
                });
                if ui.button("Apply to selected").clicked() {
                    app.push_undo();
                    let h = app.batch_elevation_mm;
                    for piece in &mut app.layout.pieces {
                        if app.selected.contains(&piece.id) {
                            piece.elevation = Some(Elevation { start: h, end: h });
                        }
                    }
                }
            });
        app.show_height_dialog = open;
    }

    if app.show_collab_dialog {
        let mut open = app.show_collab_dialog;
        egui::Window::new("Live Collaboration (WebSocket)")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Server:");
                    ui.text_edit_singleline(&mut app.collab.server_url);
                });
                ui.horizontal(|ui| {
                    ui.label("Room:");
                    ui.text_edit_singleline(&mut app.collab.room_id);
                });
                ui.horizontal(|ui| {
                    ui.label("User:");
                    ui.text_edit_singleline(&mut app.collab.user_name);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if app.collab.connected {
                        if ui.button("Disconnect").clicked() {
                            app.collab.disconnect();
                        }
                    } else if ui.button("Connect").clicked() {
                        app.collab.connect();
                    }
                    ui.weak(&app.collab.status);
                });
            });
        app.show_collab_dialog = open;
    }

    if app.show_help_dialog {
        egui::Window::new("RAILbench Shortcuts")
            .open(&mut app.show_help_dialog)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("R / Shift+R — Rotate selected track");
                ui.label("F — Toggle Power Feeder");
                ui.label("I — Toggle Isolator");
                ui.label("O — Toggle Catenary");
                ui.label("B — Toggle Bridge");
                ui.label("3 — Toggle 2D / 3D View");
                ui.label("Ctrl+Z / Ctrl+Y — Undo / Redo");
                ui.label("Ctrl+0 — Fit View");
                ui.label("Del / Backspace — Delete selected");
            });
    }
}

fn draw_track_kind_icon(painter: &egui::Painter, rect: egui::Rect, kind: &str, accent: Color32, bg: Color32) {
    painter.rect_filled(rect, 6.0, bg);
    let stroke = Stroke::new(2.2_f32, accent);
    let c = rect.center();
    let l = rect.min.x + 5.0;
    let r = rect.max.x - 5.0;
    let t = rect.min.y + 6.0;
    let b = rect.max.y - 6.0;

    match kind {
        "curve" => {
            let mut pts = Vec::with_capacity(8);
            for i in 0..=7 {
                let u = i as f32 / 7.0;
                let ang = std::f32::consts::FRAC_PI_2 * u;
                pts.push(egui::pos2(l + (r - l) * ang.sin(), b - (b - t) * (1.0 - ang.cos())));
            }
            painter.add(egui::Shape::line(pts, stroke));
        }
        "turnout-left" | "curved-turnout-left" => {
            painter.line_segment([egui::pos2(l, c.y + 3.0), egui::pos2(r, c.y + 3.0)], stroke);
            painter.line_segment([egui::pos2(l + 4.0, c.y + 3.0), egui::pos2(r - 2.0, t)], stroke);
        }
        "turnout-right" | "curved-turnout-right" => {
            painter.line_segment([egui::pos2(l, c.y - 3.0), egui::pos2(r, c.y - 3.0)], stroke);
            painter.line_segment([egui::pos2(l + 4.0, c.y - 3.0), egui::pos2(r - 2.0, b)], stroke);
        }
        "three-way" => {
            painter.line_segment([egui::pos2(l, c.y), egui::pos2(r, c.y)], stroke);
            painter.line_segment([egui::pos2(l + 5.0, c.y), egui::pos2(r - 2.0, t)], stroke);
            painter.line_segment([egui::pos2(l + 5.0, c.y), egui::pos2(r - 2.0, b)], stroke);
        }
        "crossing" | "double-slip" => {
            painter.line_segment([egui::pos2(l, t + 1.0), egui::pos2(r, b - 1.0)], stroke);
            painter.line_segment([egui::pos2(l, b - 1.0), egui::pos2(r, t + 1.0)], stroke);
        }
        "flex" => {
            let mut pts = Vec::with_capacity(9);
            for i in 0..=8 {
                let u = i as f32 / 8.0;
                let x = l + (r - l) * u;
                let y = c.y + (u * std::f32::consts::TAU).sin() * 4.5;
                pts.push(egui::pos2(x, y));
            }
            painter.add(egui::Shape::line(pts, stroke));
        }
        "buffer" => {
            painter.line_segment([egui::pos2(l, c.y), egui::pos2(r - 3.0, c.y)], stroke);
            painter.line_segment([egui::pos2(r - 3.0, t + 2.0), egui::pos2(r - 3.0, b - 2.0)], Stroke::new(2.8_f32, accent));
        }
        "turntable" => {
            painter.circle_stroke(c, 9.0, stroke);
            painter.line_segment([egui::pos2(c.x - 8.0, c.y), egui::pos2(c.x + 8.0, c.y)], stroke);
        }
        _ => {
            painter.line_segment([egui::pos2(l, c.y), egui::pos2(r, c.y)], stroke);
        }
    }
}

pub fn default_layer() -> Layer {
    Layer {
        id: "layer-1".into(),
        name: "Ebene 0".into(),
        color: Some("#38bdf8".into()),
        visible: true,
        locked: false,
        height_mm: Some(0.0),
        wood_plate_mode: None,
        wood_plate_thickness_mm: None,
        wood_plate_style: None,
        plates: None,
    }
}
