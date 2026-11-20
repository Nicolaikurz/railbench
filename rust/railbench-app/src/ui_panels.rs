use std::collections::BTreeMap;

use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Stroke};
use railbench_core::types::{Elevation, Layer};

use crate::board_tools::{self, SketchMode};
use crate::canvas2d::{parse_hex_color, ColorMode};
use crate::export::get_piece_list_summary;
use crate::i18n::{tr, Lang};
use crate::icons::{icon_button, icon_button_sized, Icon};
use crate::model_tools;
use crate::ramp_tools::{
    apply_auto_ramp, available_curves_for_catalog, generate_ramp_or_helix, min_clearance_for_scale,
    preview_auto_ramp, RampProfile, RampShape,
};
use crate::settings::{
    capture_pressed_shortcut, KeyCapture, KeyCategory, StartupAction, KEY_ACTION_DEFS,
};
use crate::terrain_tools::{self, TerrainBrushMode, TerrainRidgeProfile};
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
                            ("Shunting 0", "demotracks/shunting-0.railbench"),
                            ("Switching N", "demotracks/switching-n.railbench"),
                            ("Garden G", "demotracks/garden-g.railbench"),
                            ("Suitcase Z", "demotracks/suitcase-z.railbench"),
                        ] {
                            if ui.button(label).clicked() {
                                app.open_layout_path(path);
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        let all_demos_lbl = if app.lang == Lang::De {
                            "Alle Demo-Anlagen..."
                        } else {
                            "Demo Gallery..."
                        };
                        if ui.button(all_demos_lbl).clicked() {
                            app.show_demo_dialog = true;
                            ui.close_menu();
                        }
                    });
                    if ui.button(t.save_file).clicked() {
                        app.save_layout_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    let print_lbl = if app.lang == Lang::De {
                        "Bauplan-Export & Drucken..."
                    } else {
                        "Blueprint Export & Print..."
                    };
                    if ui.button(print_lbl).clicked() {
                        app.show_export_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button("Export PDF Blueprint...").clicked() {
                        app.export_pdf_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Export SVG Blueprint...").clicked() {
                        app.export_svg_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Export XML (.xml)...").clicked() {
                        app.export_xml_dialog();
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
                    let welcome_lbl = if app.lang == Lang::De {
                        "Startbildschirm..."
                    } else {
                        "Welcome Screen..."
                    };
                    if ui.button(welcome_lbl).clicked() {
                        app.show_welcome_dialog = true;
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
                    let copy_lbl = if app.lang == Lang::De { "Kopieren (Strg+C)" } else { "Copy (Ctrl+C)" };
                    let paste_lbl = if app.lang == Lang::De { "Einfügen (Strg+V)" } else { "Paste (Ctrl+V)" };
                    let dup_lbl = if app.lang == Lang::De { "Duplizieren (Strg+D)" } else { "Duplicate (Ctrl+D)" };
                    let sel_all_lbl = if app.lang == Lang::De { "Alles auswählen (Strg+A)" } else { "Select All (Ctrl+A)" };
                    let sel_conn_lbl = if app.lang == Lang::De { "Verbundene Gleise auswählen" } else { "Select Connected Tracks" };
                    if ui.add_enabled(!app.selected.is_empty(), egui::Button::new(copy_lbl)).clicked() {
                        app.copy_selected();
                        ui.close_menu();
                    }
                    if ui.add_enabled(app.clipboard.is_some(), egui::Button::new(paste_lbl)).clicked() {
                        app.paste_clipboard(None);
                        ui.close_menu();
                    }
                    if ui.add_enabled(!app.selected.is_empty(), egui::Button::new(dup_lbl)).clicked() {
                        app.duplicate_selected();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(sel_all_lbl).clicked() {
                        app.select_all();
                        ui.close_menu();
                    }
                    if ui.add_enabled(!app.selected.is_empty(), egui::Button::new(sel_conn_lbl)).clicked() {
                        app.select_connected_tracks();
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
                    let is_de = app.lang == Lang::De;
                    if ui.button(t.reset_view).clicked() {
                        app.pending_fit_view = true;
                        ui.close_menu();
                    }
                    let view3d_lbl = if is_de { "3D-Ansicht umschalten" } else { "Toggle 3D View" };
                    if ui.button(view3d_lbl).clicked() {
                        app.toggle_3d();
                        ui.close_menu();
                    }
                    ui.separator();
                    let mut view_changed = false;
                    view_changed |= ui.checkbox(&mut app.show_labels, t.labels).changed();
                    view_changed |= ui.checkbox(&mut app.show_connectors, t.connectors).changed();
                    view_changed |= ui
                        .checkbox(&mut app.show_rulers, if is_de { "Lineale & Raster" } else { "Rulers & Grid" })
                        .changed();
                    view_changed |= ui.checkbox(&mut app.snapping_enabled, t.snapping).changed();
                    view_changed |= ui
                        .checkbox(&mut app.show_wood_plates, if is_de { "Holzplatten anzeigen" } else { "Show Wood Plates" })
                        .changed();
                    view_changed |= ui.checkbox(&mut app.show_right_sidebar, t.layers_title).changed();
                    ui.checkbox(&mut app.power_sim_enabled, t.volt_btn);
                    view_changed |= ui.checkbox(&mut app.perf_monitor.visible, "Hardware Monitor (F3)").changed();
                    ui.separator();
                    view_changed |= ui.radio_value(&mut app.color_mode, ColorMode::Types, t.color_types).changed();
                    view_changed |= ui.radio_value(&mut app.color_mode, ColorMode::Layers, t.color_layers).changed();
                    view_changed |= ui.radio_value(&mut app.color_mode, ColorMode::Off, t.color_off).changed();
                    if view_changed {
                        app.save_current_settings();
                    }
                });

                ui.menu_button(t.menu_settings, |ui| {
                    let is_de = app.lang == Lang::De;
                    if ui.button(t.power_dialog).clicked() {
                        app.show_power_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button(t.height_dialog).clicked() {
                        app.show_height_dialog = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button(if is_de {
                            "Einstellungen & Tastenkuerzel..."
                        } else {
                            "Settings & Shortcuts..."
                        })
                        .clicked()
                    {
                        app.settings_tab = 0;
                        app.show_settings_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Tastaturkuerzel anpassen..."
                        } else {
                            "Customize Shortcuts..."
                        })
                        .clicked()
                    {
                        app.settings_tab = 1;
                        app.show_settings_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Bauplan-Druck & Export..."
                        } else {
                            "Blueprint Print & Export..."
                        })
                        .clicked()
                    {
                        app.show_export_dialog = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button(t.menu_help, |ui| {
                    let is_de = app.lang == Lang::De;
                    if ui
                        .button(if is_de {
                            "Hilfe & Handbuch..."
                        } else {
                            "Help & User Guide..."
                        })
                        .clicked()
                    {
                        app.help_tab = 0;
                        app.show_help_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Tastenkuerzel"
                        } else {
                            "Keyboard Shortcuts"
                        })
                        .clicked()
                    {
                        app.help_tab = 6;
                        app.show_help_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Demo-Anlagen..."
                        } else {
                            "Demo Layouts..."
                        })
                        .clicked()
                    {
                        app.show_demo_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(if is_de {
                            "Willkommensbildschirm..."
                        } else {
                            "Welcome Screen..."
                        })
                        .clicked()
                    {
                        app.show_welcome_dialog = true;
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
                    app.save_current_settings();
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
                            if ui.selectable_value(&mut app.rotation_step, deg, format!("{deg}°")).changed() {
                                app.save_current_settings();
                            }
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
                    app.save_current_settings();
                }
                if tb_icon(ui, Icon::Tag, app.show_labels, t.labels) {
                    app.show_labels = !app.show_labels;
                    app.save_current_settings();
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
                                app.mark_layout_dirty();
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui.button(t.power_dialog).clicked() {
                            app.show_power_dialog = true;
                            ui.close_menu();
                        }
                    });

                    // Red Short-Circuit alert pill (clicks straight to the short point!)
                    if app.power_sim_enabled {
                        let first_sc = app
                            .power_sim_cache
                            .as_ref()
                            .filter(|(_, s)| s.has_short_circuit)
                            .map(|(_, s)| (s.short_circuit_points.len(), s.short_circuit_points.first().map(|pt| (pt.x, pt.y))));
                        if let Some((cnt, maybe_pt)) = first_sc {
                            let label = if cnt > 0 { format!("⚡ SHORT ({cnt})") } else { "⚡ SHORT".into() };
                            if ui
                                .add(
                                    egui::Button::new(RichText::new(label).strong().size(11.0).color(Color32::WHITE))
                                        .fill(Color32::from_rgb(220, 38, 38))
                                        .corner_radius(CornerRadius::same(6)),
                                )
                                .on_hover_text("Focus camera on short circuit")
                                .clicked()
                            {
                                if let Some((x, y)) = maybe_pt {
                                    app.focus_world_point(x, y);
                                }
                            }
                        }
                    }

                // 3D Toggle
                if tb_icon(ui, Icon::Box3d, app.is_3d_mode, "3D View (3)") {
                    app.toggle_3d();
                }

                // Right-aligned utilities (matches TS toolbar-right: DE/EN, Collab, Theme, Layers)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if tb_icon(ui, Icon::Layers, app.show_right_sidebar, t.layers_title) {
                        app.show_right_sidebar = !app.show_right_sidebar;
                        app.save_current_settings();
                    }
                    let theme_icon = if app.dark_mode { Icon::Moon } else { Icon::Sun };
                    let theme_tip = if app.dark_mode { t.theme_dark } else { t.theme_light };
                    if tb_icon(ui, theme_icon, false, theme_tip) {
                        app.dark_mode = !app.dark_mode;
                        app.save_current_settings();
                    }
                    if tb_icon(ui, Icon::Users, app.collab.connected, "Live Collaboration") {
                        app.show_collab_dialog = !app.show_collab_dialog;
                    }
                    if pill_btn(ui, if app.lang == Lang::De { "DE" } else { "EN" }, false).clicked() {
                        app.lang = if app.lang == Lang::De { Lang::En } else { Lang::De };
                        app.save_current_settings();
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

fn draw_terrain_tab(ui: &mut egui::Ui, app: &mut RailbenchApp) {
    let p = palette(app.dark_mode);
    let is_de = app.lang == Lang::De;

    let style_label = |st: &str| -> &'static str {
        match (st, is_de) {
            ("rocky", true) => "Felsmassiv",
            ("rocky", false) => "Rocky Massif",
            ("forest", true) => "Waldgebirge",
            ("forest", false) => "Forested Ridge",
            ("meadow", true) => "Gras-Huegel",
            ("meadow", false) => "Meadow Hill",
            (_, true) => "Alpen (Schnee & Fels)",
            (_, false) => "Alpine (Snow & Rock)",
        }
    };
    let ground_label = |gt: &str| -> &'static str {
        match (gt, is_de) {
            ("water", true) => "Wasser",
            ("water", false) => "Water",
            ("sand", _) => "Sand",
            ("dirt", true) => "Erde",
            ("dirt", false) => "Dirt",
            (_, true) => "Huegel (Gras)",
            (_, false) => "Hill (Grass)",
        }
    };

    egui::ScrollArea::vertical().show(ui, |ui| {
        // 1. Unified Terrain Brush Card (5 modes: Paint / Extend / Stamp / Ground / Erase)
        ui.label(
            RichText::new(if is_de { "GELAENDE-PINSEL" } else { "TERRAIN BRUSH" })
                .strong()
                .size(11.0)
                .color(p.text_muted),
        );
        ui.add_space(4.0);

        Frame::new()
            .fill(p.bg_card)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .stroke(Stroke::new(
                if app.terrain_brush.active { 1.4_f32 } else { 1.0_f32 },
                if app.terrain_brush.active {
                    Color32::from_rgb(34, 197, 94)
                } else {
                    p.border_main
                },
            ))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let activate_mode = |app: &mut RailbenchApp, mode: TerrainBrushMode| {
                    let target = app.active_terrain_id.clone();
                    app.terrain_brush.toggle(mode, target);
                    if app.terrain_brush.active {
                        app.is_3d_mode = false;
                        app.measure_mode = false;
                        app.sketcher.stop();
                    }
                };

                let mode_btn = |ui: &mut egui::Ui,
                                active: bool,
                                lbl: &str,
                                tip: &str,
                                active_bg: Color32| -> bool {
                    let fill = if active { active_bg } else { p.bg_sidebar };
                    let tc = if active { Color32::WHITE } else { p.text_main };
                    ui.add(
                        egui::Button::new(RichText::new(lbl).strong().size(11.0).color(tc))
                            .fill(fill)
                            .stroke(Stroke::new(
                                1.0_f32,
                                if active { active_bg } else { p.border_main },
                            ))
                            .corner_radius(CornerRadius::same(6)),
                    )
                    .on_hover_text(tip)
                    .clicked()
                };

                let cur_active = app.terrain_brush.active;
                let cur_mode = app.terrain_brush.mode;

                ui.horizontal(|ui| {
                    if mode_btn(
                        ui,
                        cur_active && cur_mode == TerrainBrushMode::Paint,
                        if is_de { "Berg" } else { "Mountain" },
                        if is_de {
                            "Klicken fuer Gipfel oder ziehen fuer Gebirgskamm"
                        } else {
                            "Click for peak or drag to paint ridge"
                        },
                        Color32::from_rgb(22, 163, 74),
                    ) {
                        activate_mode(app, TerrainBrushMode::Paint);
                    }
                    if mode_btn(
                        ui,
                        cur_active && cur_mode == TerrainBrushMode::Ground,
                        if is_de { "Grund" } else { "Ground" },
                        if is_de {
                            "Wasser, Sand, Erde oder Huegel aufmalen (flach unter Gleisen & Gebaeuden)"
                        } else {
                            "Paint water, sand, dirt, or hills (flattens under buildings & tracks)"
                        },
                        Color32::from_rgb(14, 116, 144),
                    ) {
                        activate_mode(app, TerrainBrushMode::Ground);
                    }
                    if mode_btn(
                        ui,
                        cur_active && cur_mode == TerrainBrushMode::Erase,
                        if is_de { "Radieren" } else { "Erase" },
                        if is_de {
                            "Berge, Knoten oder Untergrund im 2D-Plan entfernen"
                        } else {
                            "Erase mountains, ridge nodes, or ground terrain"
                        },
                        Color32::from_rgb(185, 28, 28),
                    ) {
                        activate_mode(app, TerrainBrushMode::Erase);
                    }
                });
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    if mode_btn(
                        ui,
                        cur_active && cur_mode == TerrainBrushMode::Extend,
                        if is_de { "+ Erweitern" } else { "+ Extend" },
                        if is_de {
                            "Bestehenden Berg nahtlos mit dem Pinsel erweitern"
                        } else {
                            "Extend existing mountain ridge seamlessly"
                        },
                        p.primary_hover,
                    ) {
                        activate_mode(app, TerrainBrushMode::Extend);
                    }
                    if mode_btn(
                        ui,
                        cur_active && cur_mode == TerrainBrushMode::Stamp,
                        if is_de { "Stempel" } else { "Stamp" },
                        if is_de {
                            "Einzelklick setzt einzelnen Berggipfel"
                        } else {
                            "Single click stamps a standalone mountain peak"
                        },
                        Color32::from_rgb(180, 83, 9),
                    ) {
                        activate_mode(app, TerrainBrushMode::Stamp);
                    }
                });

                if app.terrain_brush.active {
                    ui.add_space(5.0);
                    let hint = match app.terrain_brush.mode {
                        TerrainBrushMode::Ground => {
                            if is_de {
                                "Im 2D-Plan ziehen/klicken. Shift+Rad = Radius. Rechtsklick beendet."
                            } else {
                                "Drag or click on 2D canvas. Shift+Wheel = radius. Right-click exits."
                            }
                        }
                        TerrainBrushMode::Erase => {
                            if is_de {
                                "Ueber Berge oder Untergrund ziehen zum Loeschen. Rechtsklick beendet."
                            } else {
                                "Drag over mountains or ground to erase. Right-click exits."
                            }
                        }
                        TerrainBrushMode::Extend => {
                            if is_de {
                                "Ziehen um ausgewaehlten/naechsten Berg zu erweitern. Rechtsklick beendet."
                            } else {
                                "Drag to extend selected/nearest mountain. Right-click exits."
                            }
                        }
                        TerrainBrushMode::Stamp => {
                            if is_de {
                                "Klicken um Gipfel zu stempeln. Shift+Rad = Radius. Rechtsklick beendet."
                            } else {
                                "Click to stamp peak. Shift+Wheel = radius. Right-click exits."
                            }
                        }
                        TerrainBrushMode::Paint => {
                            if is_de {
                                "Klicken (Gipfel) oder Ziehen (Kamm). Shift+Rad = Radius. Rechtsklick beendet."
                            } else {
                                "Click (peak) or drag (ridge). Shift+Wheel = radius. Right-click exits."
                            }
                        }
                    };
                    ui.label(RichText::new(hint).size(10.5).color(Color32::from_rgb(74, 222, 128)));
                }

                ui.add_space(6.0);
                match app.terrain_brush.mode {
                    TerrainBrushMode::Ground => {
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Typ:" } else { "Type:" });
                            egui::ComboBox::from_id_salt("brush_ground_type")
                                .width(155.0)
                                .selected_text(ground_label(&app.terrain_brush.ground_type))
                                .show_ui(ui, |ui| {
                                    for gt in ["hill", "water", "sand", "dirt"] {
                                        if ui
                                            .selectable_label(
                                                app.terrain_brush.ground_type == gt,
                                                ground_label(gt),
                                            )
                                            .clicked()
                                        {
                                            app.terrain_brush.set_ground_type(gt);
                                        }
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Radius:");
                            ui.add(
                                egui::DragValue::new(&mut app.terrain_brush.radius)
                                    .range(60.0..=600.0)
                                    .speed(5.0)
                                    .suffix(" mm"),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Hoehe:" } else { "Height:" });
                            ui.add(
                                egui::DragValue::new(&mut app.terrain_brush.ground_height)
                                    .range(2.0..=120.0)
                                    .speed(2.0)
                                    .suffix(" mm"),
                            );
                        });
                    }
                    TerrainBrushMode::Erase => {
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Radier-Radius:" } else { "Eraser Radius:" });
                            ui.add(
                                egui::DragValue::new(&mut app.terrain_brush.radius)
                                    .range(60.0..=600.0)
                                    .speed(5.0)
                                    .suffix(" mm"),
                            );
                        });
                    }
                    _ => {
                        ui.horizontal(|ui| {
                            ui.label("Radius:");
                            ui.add(
                                egui::DragValue::new(&mut app.terrain_brush.radius)
                                    .range(60.0..=600.0)
                                    .speed(5.0)
                                    .suffix(" mm"),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Hoehe:" } else { "Height:" });
                            ui.add(
                                egui::DragValue::new(&mut app.terrain_brush.height)
                                    .range(40.0..=500.0)
                                    .speed(5.0)
                                    .suffix(" mm"),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Biom:" } else { "Biome:" });
                            egui::ComboBox::from_id_salt("brush_mtn_style")
                                .width(155.0)
                                .selected_text(style_label(&app.terrain_brush.style))
                                .show_ui(ui, |ui| {
                                    for st in ["alpine", "rocky", "forest", "meadow"] {
                                        if ui
                                            .selectable_label(app.terrain_brush.style == st, style_label(st))
                                            .clicked()
                                        {
                                            app.terrain_brush.style = st.into();
                                        }
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Kamm:" } else { "Ridge:" });
                            let cur_rp = app.terrain_brush.ridge_profile;
                            let rp_lbl = |rp: TerrainRidgeProfile| match (rp, is_de) {
                                (TerrainRidgeProfile::Uniform, true) => "Gleichmaessig",
                                (TerrainRidgeProfile::Uniform, false) => "Uniform",
                                (TerrainRidgeProfile::PeakCenter, true) => "Mittelgipfel",
                                (TerrainRidgeProfile::PeakCenter, false) => "Center Peak",
                                (TerrainRidgeProfile::Varied, true) => "Alpen-Kamm",
                                (TerrainRidgeProfile::Varied, false) => "Alpine Peaks",
                            };
                            egui::ComboBox::from_id_salt("brush_ridge_prof")
                                .width(155.0)
                                .selected_text(rp_lbl(cur_rp))
                                .show_ui(ui, |ui| {
                                    for rp in [
                                        TerrainRidgeProfile::Uniform,
                                        TerrainRidgeProfile::PeakCenter,
                                        TerrainRidgeProfile::Varied,
                                    ] {
                                        ui.selectable_value(&mut app.terrain_brush.ridge_profile, rp, rp_lbl(rp));
                                    }
                                });
                        });
                        ui.checkbox(
                            &mut app.terrain_brush.auto_tunnel,
                            if is_de {
                                "Gleise & Strassen automatisch tunneln"
                            } else {
                                "Auto-tunnel intersecting tracks & roads"
                            },
                        );
                    }
                }
            });

        ui.add_space(10.0);

        // 2. Quick Mountain Presets & Ground Terrain Cards
        ui.label(
            RichText::new(if is_de { "BERG-VORLAGEN" } else { "MOUNTAIN PRESETS" })
                .strong()
                .size(11.0)
                .color(p.text_muted),
        );
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if is_de { "+ Berg" } else { "+ Mountain" })
                .clicked()
            {
                app.place_mountain_preset("single");
            }
            if ui
                .button(if is_de { "+ Eck-Massiv" } else { "+ Corner Massif" })
                .clicked()
            {
                app.place_mountain_preset("corner");
            }
            if ui
                .button(if is_de { "+ Kulissen-Kette" } else { "+ Backdrop Ridge" })
                .clicked()
            {
                app.place_mountain_preset("ridge");
            }
        });

        ui.add_space(8.0);
        ui.label(
            RichText::new(if is_de { "UNTERGRUND / WASSER / LANDSCHAFT" } else { "GROUND TERRAIN / WATER" })
                .strong()
                .size(11.0)
                .color(p.text_muted),
        );
        ui.add_space(4.0);
        for (gtype, name, w, d, h) in [
            ("hill", "Wiese / Rolling Meadow", 560.0, 420.0, 22.0),
            ("water", "See / Alpine Lake", 480.0, 360.0, 6.0),
            ("water", "Fluss / River", 680.0, 180.0, 6.0),
            ("sand", "Sand / Beach", 460.0, 320.0, 12.0),
            ("dirt", "Erde / Dirt Mound", 460.0, 320.0, 14.0),
        ] {
            let resp = Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(7))
                .inner_margin(Margin::symmetric(10, 6))
                .stroke(Stroke::new(1.0_f32, p.border_main))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(name).strong().size(12.0).color(p.text_main));
                    ui.label(
                        RichText::new(format!("{w:.0} x {d:.0} mm | H: {h:.0} mm"))
                            .size(10.5)
                            .color(p.text_muted),
                    );
                })
                .response
                .interact(egui::Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                app.place_ground_terrain(gtype, name, w, d, h);
            }
        }

        ui.add_space(10.0);

        // 3. Track & Road Tunnels Tool Card
        let tunneled_cnt = app
            .layout
            .pieces
            .iter()
            .filter(|pc| pc.has_tunnel == Some(true))
            .count();
        let sel_track_cnt = app
            .layout
            .pieces
            .iter()
            .filter(|pc| app.selected.contains(&pc.id))
            .count();
        Frame::new()
            .fill(p.bg_card)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .stroke(Stroke::new(1.0_f32, p.border_main))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(if is_de {
                        format!("Gleis- & Strassen-Tunnel ({tunneled_cnt} aktiv)")
                    } else {
                        format!("Track & Street Tunnels ({tunneled_cnt} active)")
                    })
                    .strong()
                    .size(11.5)
                    .color(p.text_main),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            sel_track_cnt > 0,
                            egui::Button::new(if is_de {
                                format!("Auswahl als Tunnel ({sel_track_cnt}) [T]")
                            } else {
                                format!("Toggle Selection ({sel_track_cnt}) [T]")
                            }),
                        )
                        .clicked()
                    {
                        app.toggle_selected_tunnel();
                    }
                    if ui
                        .button(if is_de { "Auto-Tunnel (Alle Berge)" } else { "Auto-Tunnel (Mountains)" })
                        .clicked()
                    {
                        app.auto_tunnel_all_mountains();
                    }
                    if tunneled_cnt > 0 && ui.button(if is_de { "Alle loesen" } else { "Clear All" }).clicked() {
                        app.clear_all_tunnels();
                    }
                });
            });

        ui.add_space(10.0);

        // 4. Terrain Items Inspector (syncs with selected terrain or active_terrain_id)
        let terrain_items: Vec<(String, String, bool)> = app
            .layout
            .models
            .as_ref()
            .map(|ms| {
                ms.iter()
                    .filter(|m| matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain")))
                    .map(|m| {
                        (
                            m.id.clone(),
                            m.name.clone(),
                            m.builtin_id.as_deref() == Some("ground-terrain"),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        if !terrain_items.is_empty() {
            if let Some((sel_tid, _, _)) = terrain_items.iter().find(|(id, _, _)| app.selected.contains(id)) {
                app.active_terrain_id = Some(sel_tid.clone());
            }
            let cur_id = app
                .active_terrain_id
                .clone()
                .filter(|aid| terrain_items.iter().any(|(id, _, _)| id == aid))
                .unwrap_or_else(|| terrain_items.last().unwrap().0.clone());
            app.active_terrain_id = Some(cur_id.clone());

            ui.label(
                RichText::new(if is_de {
                    format!("GELAENDE-INSPEKTOR ({})", terrain_items.len())
                } else {
                    format!("TERRAIN INSPECTOR ({})", terrain_items.len())
                })
                .strong()
                .size(11.0)
                .color(p.text_muted),
            );
            ui.add_space(4.0);

            let mut item_changed = false;
            let mut req_center = false;
            let mut req_delete = false;
            let mut req_smooth = false;
            let mut req_autotunnel = false;
            let mut cur_snapshot_mtn: Option<railbench_core::types::Model3D> = None;

            Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(10))
                .stroke(Stroke::new(1.0_f32, p.border_main))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if terrain_items.len() > 1 {
                        let cur_lbl = terrain_items
                            .iter()
                            .find(|(id, _, _)| id == &cur_id)
                            .map(|(_, nm, _)| nm.as_str())
                            .unwrap_or("Terrain");
                        ui.horizontal(|ui| {
                            ui.label(if is_de { "Objekt:" } else { "Item:" });
                            egui::ComboBox::from_id_salt("terrain_inspector_sel")
                                .width(155.0)
                                .selected_text(cur_lbl)
                                .show_ui(ui, |ui| {
                                    for (tid, tnm, is_gt) in &terrain_items {
                                        let disp = format!("{} {tnm}", if *is_gt { "[G]" } else { "[M]" });
                                        if ui.selectable_label(*tid == cur_id, disp).clicked() {
                                            app.active_terrain_id = Some(tid.clone());
                                            app.selected.clear();
                                            app.selected.insert(tid.clone());
                                        }
                                    }
                                });
                        });
                    }

                    if let Some(models) = &mut app.layout.models {
                        if let Some(item) = models.iter_mut().find(|m| m.id == cur_id) {
                            let is_gt = item.builtin_id.as_deref() == Some("ground-terrain");
                            if is_gt {
                                let cur_gt = item.ground_terrain_type.clone().unwrap_or_else(|| "hill".into());
                                ui.horizontal(|ui| {
                                    ui.label(if is_de { "Typ:" } else { "Type:" });
                                    egui::ComboBox::from_id_salt("insp_ground_type")
                                        .width(150.0)
                                        .selected_text(ground_label(&cur_gt))
                                        .show_ui(ui, |ui| {
                                            for gt in ["hill", "water", "sand", "dirt"] {
                                                if ui.selectable_label(cur_gt == gt, ground_label(gt)).clicked() {
                                                    item.ground_terrain_type = Some(gt.into());
                                                    item_changed = true;
                                                }
                                            }
                                        });
                                });
                            } else {
                                let cur_st = item.mountain_style.clone().unwrap_or_else(|| "alpine".into());
                                ui.horizontal(|ui| {
                                    ui.label(if is_de { "Biom:" } else { "Biome:" });
                                    egui::ComboBox::from_id_salt("insp_mtn_style")
                                        .width(150.0)
                                        .selected_text(style_label(&cur_st))
                                        .show_ui(ui, |ui| {
                                            for st in ["alpine", "rocky", "forest", "meadow"] {
                                                if ui.selectable_label(cur_st == st, style_label(st)).clicked() {
                                                    item.mountain_style = Some(st.into());
                                                    app.terrain_brush.style = st.into();
                                                    item_changed = true;
                                                }
                                            }
                                        });
                                });
                            }

                            ui.horizontal(|ui| {
                                ui.label("X:");
                                item_changed |= ui.add(egui::DragValue::new(&mut item.transform.x).speed(5.0).suffix(" mm")).changed();
                                ui.label("Y:");
                                item_changed |= ui.add(egui::DragValue::new(&mut item.transform.y).speed(5.0).suffix(" mm")).changed();
                            });
                            ui.horizontal(|ui| {
                                ui.label(if is_de { "Breite:" } else { "Width:" });
                                item_changed |= ui.add(egui::DragValue::new(&mut item.width).range(60.0..=6000.0).speed(10.0).suffix(" mm")).changed();
                            });
                            ui.horizontal(|ui| {
                                ui.label(if is_de { "Tiefe:" } else { "Depth:" });
                                item_changed |= ui.add(egui::DragValue::new(&mut item.depth).range(60.0..=6000.0).speed(10.0).suffix(" mm")).changed();
                            });
                            let (h_min, h_max) = if is_gt { (2.0, 120.0) } else { (10.0, 600.0) };
                            let prev_h = item.height.max(1.0);
                            ui.horizontal(|ui| {
                                ui.label(if is_de { "Hoehe:" } else { "Height:" });
                                if ui.add(egui::DragValue::new(&mut item.height).range(h_min..=h_max).speed(4.0).suffix(" mm")).changed() {
                                    let ratio = item.height / prev_h;
                                    if let Some(nodes) = &mut item.mountain_path {
                                        for n in nodes.iter_mut() {
                                            n.height = (n.height * ratio).round().clamp(h_min, h_max);
                                        }
                                    }
                                    item_changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label(if is_de { "Drehung:" } else { "Rotation:" });
                                if ui.add(egui::DragValue::new(&mut item.transform.rotation).speed(2.0).suffix(" deg")).changed() {
                                    item.transform.rotation = item.transform.rotation.rem_euclid(360.0);
                                    item_changed = true;
                                }
                            });

                            if !is_gt {
                                if item.mountain_path.as_ref().map_or(false, |p| p.len() >= 3) {
                                    if ui
                                        .button(if is_de { "Gebirgskamm glaetten" } else { "Smooth Mountain Ridge" })
                                        .clicked()
                                    {
                                        req_smooth = true;
                                    }
                                }
                                cur_snapshot_mtn = Some(item.clone());
                            }
                        }
                    }

                    if let Some(ref m_snap) = cur_snapshot_mtn {
                        let col_cnt = terrain_tools::count_mountain_tunnel_collisions(
                            m_snap,
                            &app.layout,
                            &app.catalogs,
                        );
                        ui.add_space(3.0);
                        ui.horizontal(|ui| {
                            let status_col = if col_cnt > 0 {
                                Color32::from_rgb(16, 185, 129)
                            } else {
                                p.text_muted
                            };
                            ui.label(
                                RichText::new(if col_cnt > 0 {
                                    if is_de {
                                        format!("Tunnel aktiv ({col_cnt} Abschnitte)")
                                    } else {
                                        format!("Tunnel active ({col_cnt} span(s))")
                                    }
                                } else if is_de {
                                    "Keine Gleiskreuzung (Massiv)".to_string()
                                } else {
                                    "Solid mountain (no crossings)".to_string()
                                })
                                .size(11.0)
                                .color(status_col),
                            );
                            if ui.small_button("Auto-Tunnel").clicked() {
                                req_autotunnel = true;
                            }
                        });
                    }

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(if is_de { "Auf Tisch zentrieren" } else { "Center on Table" })
                            .clicked()
                        {
                            req_center = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new(if is_de { "Loeschen" } else { "Delete" }).color(Color32::WHITE))
                                    .fill(Color32::from_rgb(185, 28, 28)),
                            )
                            .clicked()
                        {
                            req_delete = true;
                        }
                    });
                });

            if req_smooth {
                app.push_undo();
                if let Some(models) = &mut app.layout.models {
                    if let Some(m) = models.iter_mut().find(|m| m.id == cur_id) {
                        if terrain_tools::smooth_mountain_path(m) {
                            item_changed = true;
                        }
                    }
                }
            }
            if req_autotunnel {
                if let Some(m_clone) = cur_snapshot_mtn {
                    app.push_undo();
                    terrain_tools::apply_auto_tunnel_for_mountain(&m_clone, &mut app.layout);
                    item_changed = true;
                }
            }
            if req_center {
                app.center_terrain_item_on_board(&cur_id);
            }
            if req_delete {
                app.push_undo();
                if let Some(models) = &mut app.layout.models {
                    models.retain(|m| m.id != cur_id);
                }
                app.selected.remove(&cur_id);
                app.active_terrain_id = None;
                app.mark_layout_dirty();
            } else if item_changed {
                app.mark_layout_dirty();
            }
        }
    });
}

fn draw_board_tab(ui: &mut egui::Ui, app: &mut RailbenchApp) {
    let p = palette(app.dark_mode);
    let is_de = app.lang == Lang::De;
    board_tools::ensure_baseboards(&mut app.layout);

    let boards_snapshot = board_tools::get_baseboards_slice(&app.layout);
    let active_id = app
        .active_board_id
        .clone()
        .filter(|id| boards_snapshot.iter().any(|b| b.id.as_deref() == Some(id.as_str())))
        .or_else(|| boards_snapshot.first().and_then(|b| b.id.clone()))
        .unwrap_or_else(|| "board-1".into());
    app.active_board_id = Some(active_id.clone());

    let mut changed = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // 0. Active Sketcher Banner
        if app.sketcher.active {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(14, 116, 144, 55))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(10))
                .stroke(Stroke::new(1.4_f32, Color32::from_rgb(56, 189, 248)))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(if is_de { "CAD PLATTEN-SKIZZIERER AKTIV" } else { "CAD BOARD SKETCHER ACTIVE" })
                            .strong()
                            .size(11.5)
                            .color(Color32::from_rgb(56, 189, 248)),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        for (mode, lbl) in [
                            (SketchMode::Poly, "Poly"),
                            (SketchMode::Rect, "Rect"),
                            (SketchMode::Edit, "Edit"),
                        ] {
                            if ui.selectable_label(app.sketcher.mode == mode, lbl).clicked() {
                                app.sketcher.set_mode(&app.layout, mode);
                            }
                        }
                    });
                    ui.add_space(3.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Grid:").size(10.5).color(p.text_sub));
                        for (step, lbl) in [
                            (10.0, "10"),
                            (25.0, "25"),
                            (50.0, "50"),
                            (100.0, "100"),
                            (0.0, "Off"),
                        ] {
                            let active = (app.sketcher.snap_step_mm - step).abs() < 0.1;
                            if ui.selectable_label(active, lbl).clicked() {
                                app.sketcher.snap_step_mm = step;
                            }
                        }
                    });
                    ui.checkbox(&mut app.sketcher.ortho_snap, "Ortho Snap");
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if app.sketcher.mode == SketchMode::Poly && app.sketcher.points.len() >= 3 {
                            if ui.button(if is_de { "Fertig" } else { "Finish" }).clicked() {
                                let pts = app.sketcher.points.clone();
                                app.push_undo();
                                if let Some(bid) = app.sketcher.commit_points(
                                    &mut app.layout,
                                    &mut app.next_piece_seq,
                                    &pts,
                                    false,
                                    true,
                                ) {
                                    app.active_board_id = Some(bid);
                                }
                                app.sketcher.stop();
                                changed = true;
                            }
                        }
                        if app.sketcher.mode == SketchMode::Poly && !app.sketcher.points.is_empty() {
                            if ui.button("Undo Pt").clicked() {
                                app.sketcher.points.pop();
                            }
                        }
                        if ui.button(if is_de { "Abbrechen" } else { "Cancel" }).clicked() {
                            app.sketcher.stop();
                        }
                    });
                });
            ui.add_space(6.0);
        }

        // 1. Multi-Table Baseboard Selector Card
        ui.label(RichText::new("TABLES / GRUNDPLATTEN").strong().size(11.0).color(p.text_muted));
        ui.add_space(3.0);
        Frame::new()
            .fill(p.bg_card)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .stroke(Stroke::new(1.0_f32, p.border_main))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if ui.button("+ Table").clicked() {
                        app.push_undo();
                        let new_id = board_tools::add_baseboard(&mut app.layout, &mut app.next_piece_seq);
                        app.active_board_id = Some(new_id);
                        changed = true;
                    }
                    if ui.button("Sketch").on_hover_text("Draw a new custom table polygon").clicked() {
                        app.is_3d_mode = false;
                        app.terrain_brush.set_active(false, None, None);
                        app.sketcher.start_table_sketch(&app.layout, SketchMode::Poly, None, true);
                    }
                    if ui.button("Dup").on_hover_text("Duplicate selected table").clicked() {
                        app.push_undo();
                        if let Some(new_id) = board_tools::duplicate_baseboard(&mut app.layout, &active_id, &mut app.next_piece_seq) {
                            app.active_board_id = Some(new_id);
                            changed = true;
                        }
                    }
                    if boards_snapshot.len() > 1 && ui.button("Del").clicked() {
                        app.push_undo();
                        if board_tools::remove_baseboard(&mut app.layout, &active_id) {
                            app.active_board_id = board_tools::ensure_baseboards(&mut app.layout)
                                .first()
                                .and_then(|b| b.id.clone());
                            changed = true;
                        }
                    }
                });

                ui.add_space(4.0);
                let active_label = boards_snapshot
                    .iter()
                    .find(|b| b.id.as_deref() == Some(active_id.as_str()))
                    .and_then(|b| b.name.as_deref())
                    .unwrap_or("Tisch");
                egui::ComboBox::from_id_salt("board_table_selector")
                    .width(ui.available_width())
                    .selected_text(active_label)
                    .show_ui(ui, |ui| {
                        for b in &boards_snapshot {
                            let bid = b.id.as_deref().unwrap_or("board-1");
                            let lbl = format!(
                                "{} ({:.0}x{:.0})",
                                b.name.as_deref().unwrap_or(bid),
                                b.width,
                                b.depth
                            );
                            if ui.selectable_label(bid == active_id, lbl).clicked() {
                                app.active_board_id = Some(bid.to_string());
                            }
                        }
                    });

                let cur_id = app.active_board_id.clone().unwrap_or(active_id.clone());
                if let Some(bs) = app.layout.baseboards.as_mut() {
                    if let Some(board) = bs.iter_mut().find(|b| b.id.as_deref() == Some(cur_id.as_str())) {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            let name_buf = board.name.get_or_insert_with(|| "Tisch".into());
                            changed |= ui.add(egui::TextEdit::singleline(name_buf).desired_width(ui.available_width())).changed();
                        });
                        changed |= ui.checkbox(&mut board.enabled, if is_de { "Platte sichtbar" } else { "Table visible" }).changed();
                    }
                }
            });

        ui.add_space(6.0);

        // 2. Shape & Dimensions Card
        ui.label(RichText::new("SHAPE & DIMENSIONS").strong().size(11.0).color(p.text_muted));
        ui.add_space(3.0);
        let cur_id = app.active_board_id.clone().unwrap_or(active_id.clone());
        let mut req_sketch: Option<SketchMode> = None;
        let mut req_fit_tracks = false;
        let mut req_snap_right = false;

        Frame::new()
            .fill(p.bg_card)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .stroke(Stroke::new(1.0_f32, p.border_main))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("CAD:").size(11.0).color(p.text_sub));
                    if ui.small_button("Poly").clicked() { req_sketch = Some(SketchMode::Poly); }
                    if ui.small_button("Rect").clicked() { req_sketch = Some(SketchMode::Rect); }
                    if ui.small_button("Edit").clicked() { req_sketch = Some(SketchMode::Edit); }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Fit Tracks").clicked() { req_fit_tracks = true; }
                    });
                });
                ui.add_space(4.0);

                if let Some(bs) = app.layout.baseboards.as_mut() {
                    if let Some(board) = bs.iter_mut().find(|b| b.id.as_deref() == Some(cur_id.as_str())) {
                        let cur_shape = board.shape.clone().unwrap_or_else(|| "rect".into());
                        let shape_lbl = match cur_shape.as_str() {
                            "l-shape" => "L-Shape",
                            "u-shape" => "U-Shape",
                            "t-shape" => "T-Shape",
                            "triangle" => "Triangle",
                            "custom" => "Custom Polygon",
                            _ => "Rectangle",
                        };
                        ui.horizontal(|ui| {
                            ui.label("Shape:");
                            egui::ComboBox::from_id_salt("board_shape_combo")
                                .width(ui.available_width())
                                .selected_text(shape_lbl)
                                .show_ui(ui, |ui| {
                                    for (val, lbl) in [
                                        ("rect", "Rectangle"),
                                        ("l-shape", "L-Shape"),
                                        ("u-shape", "U-Shape"),
                                        ("t-shape", "T-Shape"),
                                        ("triangle", "Triangle"),
                                        ("custom", "Custom Polygon"),
                                    ] {
                                        if ui.selectable_label(cur_shape == val, lbl).clicked() {
                                            board.shape = Some(val.into());
                                            board_tools::apply_shape_defaults(board);
                                            changed = true;
                                        }
                                    }
                                });
                        });

                        ui.horizontal(|ui| {
                            ui.label("Width:");
                            changed |= ui.add(egui::DragValue::new(&mut board.width).range(200.0..=15000.0).speed(20.0).suffix(" mm")).changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label("Depth:");
                            changed |= ui.add(egui::DragValue::new(&mut board.depth).range(200.0..=10000.0).speed(20.0).suffix(" mm")).changed();
                        });

                        match board.shape.as_deref().unwrap_or("rect") {
                            "l-shape" => {
                                let ww = board.l_wing_width.get_or_insert(800.0);
                                ui.horizontal(|ui| {
                                    ui.label("Wing W:");
                                    changed |= ui.add(egui::DragValue::new(ww).range(200.0..=10000.0).speed(10.0).suffix(" mm")).changed();
                                });
                                let wd = board.l_wing_depth.get_or_insert(800.0);
                                ui.horizontal(|ui| {
                                    ui.label("Wing D:");
                                    changed |= ui.add(egui::DragValue::new(wd).range(200.0..=10000.0).speed(10.0).suffix(" mm")).changed();
                                });
                                let side = board.l_side.get_or_insert_with(|| "left".into());
                                ui.horizontal(|ui| {
                                    ui.label("Side:");
                                    changed |= ui.selectable_value(side, "left".into(), "Left").changed();
                                    changed |= ui.selectable_value(side, "right".into(), "Right").changed();
                                });
                            }
                            "u-shape" => {
                                let lw = board.u_left_wing_width.get_or_insert(700.0);
                                ui.horizontal(|ui| {
                                    ui.label("Left W:");
                                    changed |= ui.add(egui::DragValue::new(lw).range(200.0..=8000.0).speed(10.0).suffix(" mm")).changed();
                                });
                                let rw = board.u_right_wing_width.get_or_insert(700.0);
                                ui.horizontal(|ui| {
                                    ui.label("Right W:");
                                    changed |= ui.add(egui::DragValue::new(rw).range(200.0..=8000.0).speed(10.0).suffix(" mm")).changed();
                                });
                                let bd = board.u_back_depth.get_or_insert(600.0);
                                ui.horizontal(|ui| {
                                    ui.label("Back D:");
                                    changed |= ui.add(egui::DragValue::new(bd).range(200.0..=8000.0).speed(10.0).suffix(" mm")).changed();
                                });
                            }
                            "t-shape" => {
                                let td = board.t_bar_depth.get_or_insert(600.0);
                                ui.horizontal(|ui| {
                                    ui.label("Bar D:");
                                    changed |= ui.add(egui::DragValue::new(td).range(200.0..=8000.0).speed(10.0).suffix(" mm")).changed();
                                });
                                let sw = board.t_stem_width.get_or_insert(800.0);
                                ui.horizontal(|ui| {
                                    ui.label("Stem W:");
                                    changed |= ui.add(egui::DragValue::new(sw).range(200.0..=8000.0).speed(10.0).suffix(" mm")).changed();
                                });
                            }
                            "triangle" => {
                                let ttype = board.triangle_type.get_or_insert_with(|| "corner-tl".into());
                                egui::ComboBox::from_id_salt("board_tri_type")
                                    .width(ui.available_width())
                                    .selected_text(ttype.as_str())
                                    .show_ui(ui, |ui| {
                                        for opt in [
                                            "corner-tl", "corner-tr", "corner-bl", "corner-br",
                                            "iso-top", "iso-bottom", "iso-left", "iso-right",
                                        ] {
                                            if ui.selectable_label(ttype.as_str() == opt, opt).clicked() {
                                                *ttype = opt.into();
                                                changed = true;
                                            }
                                        }
                                    });
                            }
                            "custom" => {
                                let pt_cnt = board.custom_polygon.as_ref().map(|v| v.len()).unwrap_or(0);
                                ui.label(RichText::new(format!("Custom vertices: {pt_cnt}")).size(10.5).color(p.text_muted));
                            }
                            _ => {}
                        }

                        ui.add_space(4.0);
                        ui.label(RichText::new("Presets").size(10.5).color(p.text_muted));
                        ui.horizontal_wrapped(|ui| {
                            for (lbl, w, d) in [
                                ("2000x1000", 2000.0, 1000.0),
                                ("2400x1200", 2400.0, 1200.0),
                                ("3000x1500", 3000.0, 1500.0),
                            ] {
                                if ui.small_button(lbl).clicked() {
                                    board.width = w;
                                    board.depth = d;
                                    board.enabled = true;
                                    changed = true;
                                }
                            }
                        });
                    }
                }
            });

        if let Some(mode) = req_sketch {
            app.is_3d_mode = false;
            app.terrain_brush.set_active(false, None, None);
            app.sketcher.start_table_sketch(&app.layout, mode, Some(cur_id.clone()), false);
        }
        if req_fit_tracks {
            app.push_undo();
            let layout_copy = app.layout.clone();
            if let Some(bs) = app.layout.baseboards.as_mut() {
                if let Some(bb) = bs.iter_mut().find(|b| b.id.as_deref() == Some(cur_id.as_str())) {
                    changed |= board_tools::fit_board_to_tracks(&layout_copy, bb);
                }
            }
        }

        ui.add_space(6.0);

        // 3. Position & Surface Card
        ui.label(RichText::new("POSITION & SURFACE").strong().size(11.0).color(p.text_muted));
        ui.add_space(3.0);
        Frame::new()
            .fill(p.bg_card)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .stroke(Stroke::new(1.0_f32, p.border_main))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if let Some(bs) = app.layout.baseboards.as_mut() {
                    if let Some(board) = bs.iter_mut().find(|b| b.id.as_deref() == Some(cur_id.as_str())) {
                        ui.horizontal(|ui| {
                            ui.label("Origin:");
                            for (pm, lbl) in [("center", "Center"), ("top-left", "Top-Left"), ("custom", "Custom")] {
                                if ui.selectable_label(board.position_mode == pm, lbl).clicked() {
                                    board.position_mode = pm.into();
                                    changed = true;
                                }
                            }
                        });
                        if board.position_mode == "custom" {
                            ui.horizontal(|ui| {
                                let cx = board.custom_x.get_or_insert(0.0);
                                ui.label("X:");
                                changed |= ui.add(egui::DragValue::new(cx).speed(10.0).suffix(" mm")).changed();
                                let cy = board.custom_y.get_or_insert(0.0);
                                ui.label("Y:");
                                changed |= ui.add(egui::DragValue::new(cy).speed(10.0).suffix(" mm")).changed();
                            });
                        }
                        if boards_snapshot.len() > 1 {
                            if ui.small_button("Snap Right").clicked() {
                                req_snap_right = true;
                            }
                        }

                        ui.horizontal(|ui| {
                            ui.label("Style:");
                            egui::ComboBox::from_id_salt("board_style_combo")
                                .width(ui.available_width())
                                .selected_text(&board.style)
                                .show_ui(ui, |ui| {
                                    for st in ["wood", "dark-wood", "plywood", "slate", "white", "grass"] {
                                        if ui.selectable_label(board.style == st, st).clicked() {
                                            board.style = st.into();
                                            changed = true;
                                        }
                                    }
                                });
                        });

                        let rot = board.rotation.get_or_insert(0.0);
                        ui.horizontal(|ui| {
                            ui.label("Rot:");
                            changed |= ui.add(egui::DragValue::new(rot).range(-180.0..=180.0).speed(1.0).suffix(" deg")).changed();
                            if ui.small_button("-45").clicked() { *rot = (*rot - 45.0).clamp(-180.0, 180.0); changed = true; }
                            if ui.small_button("0").clicked() { *rot = 0.0; changed = true; }
                            if ui.small_button("+45").clicked() { *rot = (*rot + 45.0).clamp(-180.0, 180.0); changed = true; }
                        });

                        let cr = board.corner_radius.get_or_insert(12.0);
                        ui.horizontal(|ui| {
                            ui.label("Radius:");
                            changed |= ui.add(egui::DragValue::new(cr).range(0.0..=120.0).speed(1.0).suffix(" mm")).changed();
                        });
                        let show_dim = board.show_dimensions.get_or_insert(true);
                        changed |= ui.checkbox(show_dim, "Show Dimensions").changed();
                    }
                }
            });

        if req_snap_right {
            if let Some(other) = boards_snapshot.iter().find(|b| b.enabled && b.id.as_deref() != Some(cur_id.as_str())) {
                let ob = board_tools::get_baseboard_bounds(other);
                app.push_undo();
                if let Some(bs) = app.layout.baseboards.as_mut() {
                    if let Some(bb) = bs.iter_mut().find(|b| b.id.as_deref() == Some(cur_id.as_str())) {
                        bb.position_mode = "custom".into();
                        bb.custom_x = Some((ob.x + ob.width).round());
                        bb.custom_y = Some(ob.y.round());
                        changed = true;
                    }
                }
            }
        }

        ui.add_space(6.0);

        // 4. Per-Layer Wood Sub-Plates Card
        ui.label(RichText::new("PER-LAYER WOOD PLATES").strong().size(11.0).color(p.text_muted));
        ui.add_space(3.0);
        Frame::new()
            .fill(p.bg_card)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .stroke(Stroke::new(1.0_f32, p.border_main))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.checkbox(&mut app.show_wood_plates, if is_de { "Holzplatten anzeigen (2D/3D)" } else { "Show Wood Plates (2D/3D)" });
                ui.add_space(4.0);

                let ref_board = boards_snapshot
                    .iter()
                    .find(|b| b.id.as_deref() == Some(cur_id.as_str()))
                    .or_else(|| boards_snapshot.first())
                    .cloned();
                let mut plate_sketch_req: Option<(String, String, Option<String>, SketchMode, bool)> = None;

                if let Some(layers) = app.layout.layers.as_mut() {
                    for layer in layers.iter_mut() {
                        let lid = layer.id.clone();
                        let h = layer.height_mm.unwrap_or(0.0);
                        ui.collapsing(format!("{} ({h:.0} mm)", layer.name), |ui| {
                            let mode = layer.wood_plate_mode.get_or_insert_with(|| {
                                if h.abs() < 1e-6 { "table".into() } else { "none".into() }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Mode:");
                                for (mv, lbl) in [("table", "Table"), ("tight", "Trackbed"), ("none", "None")] {
                                    if ui.selectable_label(mode.as_str() == mv, lbl).clicked() {
                                        *mode = mv.into();
                                        changed = true;
                                    }
                                }
                            });

                            let thick = layer.wood_plate_thickness_mm.get_or_insert(12.0);
                            ui.horizontal(|ui| {
                                ui.label("Thick:");
                                changed |= ui.add(egui::DragValue::new(thick).range(3.0..=40.0).speed(0.5).suffix(" mm")).changed();
                            });

                            let lstyle = layer.wood_plate_style.get_or_insert_with(|| "plywood".into());
                            ui.horizontal(|ui| {
                                ui.label("Style:");
                                egui::ComboBox::from_id_salt(format!("layer_pstyle_{lid}"))
                                    .width(120.0)
                                    .selected_text(lstyle.as_str())
                                    .show_ui(ui, |ui| {
                                        for st in ["plywood", "wood", "dark-wood", "slate", "white", "grass"] {
                                            if ui.selectable_label(lstyle.as_str() == st, st).clicked() {
                                                *lstyle = st.into();
                                                changed = true;
                                            }
                                        }
                                    });
                            });

                            ui.horizontal(|ui| {
                                if ui.small_button("+ Plate").clicked() {
                                    if let Some(ref tb) = ref_board {
                                        let cnt = layer.plates.as_ref().map(|v| v.len()).unwrap_or(0) + 1;
                                        let np = board_tools::make_layer_plate(tb, layer, format!("Platte {cnt}"), &mut app.next_piece_seq);
                                        layer.plates.get_or_insert_with(Vec::new).push(np);
                                        board_tools::sync_wood_mode(layer);
                                        changed = true;
                                    }
                                }
                                if ui.small_button("Sketch Plate").clicked() {
                                    plate_sketch_req = Some((lid.clone(), cur_id.clone(), None, SketchMode::Poly, true));
                                }
                                if ui.small_button("Rect Plate").clicked() {
                                    plate_sketch_req = Some((lid.clone(), cur_id.clone(), None, SketchMode::Rect, true));
                                }
                            });

                            let mut del_plate_idx: Option<usize> = None;
                            if let Some(plates) = layer.plates.as_mut() {
                                for (pi, plate) in plates.iter_mut().enumerate() {
                                    let pid = plate.id.clone();
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        let pname = plate.name.get_or_insert_with(|| pid.clone());
                                        changed |= ui.add(egui::TextEdit::singleline(pname).desired_width(110.0)).changed();
                                        if ui.small_button("Fit Table").clicked() {
                                            if let Some(ref tb) = ref_board {
                                                board_tools::copy_board_shape_to_plate(tb, plate);
                                                changed = true;
                                            }
                                        }
                                        if ui.small_button("Edit").clicked() {
                                            plate_sketch_req = Some((lid.clone(), cur_id.clone(), Some(pid.clone()), SketchMode::Edit, false));
                                        }
                                        if ui.small_button("Del").clicked() {
                                            del_plate_idx = Some(pi);
                                        }
                                    });
                                    let pshape = plate.shape.get_or_insert_with(|| "rect".into()).clone();
                                    ui.horizontal(|ui| {
                                        ui.label("Shape:");
                                        egui::ComboBox::from_id_salt(format!("plate_shape_{pid}"))
                                            .width(105.0)
                                            .selected_text(pshape.as_str())
                                            .show_ui(ui, |ui| {
                                                for s in ["rect", "l-shape", "u-shape", "t-shape", "triangle", "custom"] {
                                                    if ui.selectable_label(pshape.as_str() == s, s).clicked() {
                                                        plate.shape = Some(s.into());
                                                        board_tools::apply_plate_shape_defaults(plate);
                                                        changed = true;
                                                    }
                                                }
                                            });
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("W:");
                                        changed |= ui.add(egui::DragValue::new(&mut plate.width).range(100.0..=15000.0).speed(10.0)).changed();
                                        ui.label("D:");
                                        changed |= ui.add(egui::DragValue::new(&mut plate.depth).range(100.0..=10000.0).speed(10.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        let px = plate.x.get_or_insert(0.0);
                                        ui.label("X:");
                                        changed |= ui.add(egui::DragValue::new(px).speed(10.0)).changed();
                                        let py = plate.y.get_or_insert(0.0);
                                        ui.label("Y:");
                                        changed |= ui.add(egui::DragValue::new(py).speed(10.0)).changed();
                                    });
                                }
                            }
                            if let Some(di) = del_plate_idx {
                                if let Some(plates) = layer.plates.as_mut() {
                                    plates.remove(di);
                                }
                                board_tools::sync_wood_mode(layer);
                                changed = true;
                            }
                        });
                    }
                }

                if let Some((lid, bid, pid, mode, create_new)) = plate_sketch_req {
                    app.is_3d_mode = false;
                    app.terrain_brush.set_active(false, None, None);
                    app.sketcher.start_plate_sketch(&app.layout, mode, lid, bid, pid, create_new);
                }
            });
    });

    if changed {
        board_tools::sync_primary_baseboard(&mut app.layout);
        app.mark_layout_dirty();
    }
}

fn draw_bom_tab(ui: &mut egui::Ui, app: &mut RailbenchApp) {
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
                draw_bom_piece_card(ui, p, it);
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
                draw_bom_piece_card(ui, p, it);
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
}

fn draw_bom_piece_card(ui: &mut egui::Ui, p: crate::theme::RailbenchPalette, it: &crate::export::PieceSummaryItem) {
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
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&it.catalog_id).strong().size(12.0).color(p.text_main));
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
                });
            });
        });
}

pub fn draw_right_sidebar(ctx: &egui::Context, app: &mut RailbenchApp, viewport_rect: egui::Rect) {
    if !app.show_right_sidebar {
        return;
    }
    let t = tr(app.lang);
    let p = palette(app.dark_mode);
    let bg_alpha = if app.dark_mode { 194 } else { 212 };
    let card_alpha = if app.dark_mode { 180 } else { 198 };
    let card_fill = Color32::from_rgba_unmultiplied(p.bg_card.r(), p.bg_card.g(), p.bg_card.b(), card_alpha);
    let win_fill = Color32::from_rgba_unmultiplied(p.bg_sidebar.r(), p.bg_sidebar.g(), p.bg_sidebar.b(), bg_alpha);
    let win_border = Color32::from_rgba_unmultiplied(p.border_main.r(), p.border_main.g(), p.border_main.b(), 215);

    let mut open = app.show_right_sidebar;
    let mut visibility_or_layers_changed = false;
    let max_w = (viewport_rect.width() * 0.45).max(260.0);
    let max_h = (viewport_rect.height() - 56.0).max(160.0);
    let def_pos = egui::pos2(
        (viewport_rect.max.x - 274.0).max(viewport_rect.min.x + 12.0),
        viewport_rect.min.y + 12.0,
    );

    egui::Window::new(RichText::new(t.layers_title.to_uppercase()).strong().size(11.5).color(p.text_main))
        .id(egui::Id::new("floating_layers_panel"))
        .open(&mut open)
        .collapsible(true)
        .resizable([true, true])
        .default_width(256.0)
        .min_width(215.0)
        .max_width(max_w)
        .max_height(max_h)
        .default_pos(def_pos)
        .constrain_to(viewport_rect.shrink(6.0))
        .frame(
            Frame::new()
                .fill(win_fill)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(10))
                .stroke(Stroke::new(1.0_f32, win_border))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 6],
                    blur: 18,
                    spread: 0,
                    color: Color32::from_black_alpha(if app.dark_mode { 115 } else { 55 }),
                }),
        )
        .show(ctx, |ui| {
            // Count pieces per layer
            let mut layer_counts: BTreeMap<String, usize> = BTreeMap::new();
            for piece in &app.layout.pieces {
                let lid = piece.layer_id.clone().unwrap_or_else(|| "layer-1".into());
                *layer_counts.entry(lid).or_insert(0) += 1;
            }

            let active_id = app.layout.active_layer_id.clone().unwrap_or_else(|| "layer-1".into());
            let mut delete_layer_id: Option<String> = None;
            let total_layers = app.layout.layers.as_ref().map(|l| l.len()).unwrap_or(0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .max_height((viewport_rect.height() - 190.0).max(96.0))
                .show(ui, |ui| {
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
                                .fill(card_fill)
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
                                                        visibility_or_layers_changed = true;
                                                    }
                                                });
                                            });
                                        });
                                    });
                                });
                        }
                    }
                });

            ui.add_space(6.0);
            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new(RichText::new(t.add_layer_full).strong().size(12.0).color(p.text_main))
                        .fill(card_fill)
                        .stroke(Stroke::new(1.0_f32, p.border_main))
                        .corner_radius(CornerRadius::same(7)),
                )
                .clicked()
            {
                app.add_layer();
            }

            ui.add_space(6.0);
            let orphaned_count = app.count_orphaned_items();
            if orphaned_count > 0 {
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(239, 68, 68, 30))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(8))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(239, 68, 68, 110)))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let warn_txt = if app.lang == Lang::De {
                            format!("{orphaned_count} Element(e) auf gelöschter Ebene")
                        } else {
                            format!("{orphaned_count} item(s) on deleted layer")
                        };
                        ui.label(RichText::new(warn_txt).strong().size(11.0).color(Color32::from_rgb(248, 113, 113)));
                        let btn_lbl = if app.lang == Lang::De {
                            "Aktiver Ebene zuweisen"
                        } else {
                            "Reassign to active layer"
                        };
                        if ui
                            .add(
                                egui::Button::new(RichText::new(btn_lbl).strong().size(11.0).color(Color32::WHITE))
                                    .fill(p.primary_hover)
                                    .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            app.reassign_orphaned_items();
                        }
                    });
                ui.add_space(6.0);
            }

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

            if let Some(del_id) = delete_layer_id {
                app.push_undo();
                if let Some(layers) = &mut app.layout.layers {
                    layers.retain(|l| l.id != del_id);
                    if app.layout.active_layer_id.as_deref() == Some(del_id.as_str()) {
                        app.layout.active_layer_id = layers.first().map(|l| l.id.clone());
                    }
                }
                visibility_or_layers_changed = true;
            }
        });

    if !open && app.show_right_sidebar {
        app.show_right_sidebar = false;
        app.save_current_settings();
    }
    if visibility_or_layers_changed {
        app.mark_layout_dirty();
    }
}

/// Floating pill bar at bottom-center of canvas when 1+ tracks or models are selected (matches src/ui/selection-bar.ts).
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
    let layers_list: Vec<(String, String)> = app
        .layout
        .layers
        .as_ref()
        .map(|ls| ls.iter().map(|l| (l.id.clone(), l.name.clone())).collect())
        .unwrap_or_else(|| vec![("layer-1".into(), "Ebene 0".into())]);

    let first_sel = app
        .layout
        .pieces
        .iter()
        .find(|pc| app.selected.contains(&pc.id))
        .cloned();

    let Some(sample) = first_sel else {
        // Selected item(s) are 3D buildings/models: show compact model selection bar
        let sel_models: Vec<_> = app
            .layout
            .models
            .as_ref()
            .map(|ms| ms.iter().filter(|m| app.selected.contains(&m.id)).cloned().collect())
            .unwrap_or_default();
        let Some(model) = sel_models.first() else { return };
        let m_lid = model.layer_id.as_deref().unwrap_or("layer-1");
        let all_same_layer = sel_models.iter().all(|m| m.layer_id.as_deref().unwrap_or("layer-1") == m_lid);
        let layer_exists = layers_list.iter().any(|(id, _)| id == m_lid);
        let cur_layer_label = if !layer_exists {
            if app.lang == Lang::De { "(!) Ebene fehlt" } else { "(!) Missing layer" }.to_string()
        } else if !all_same_layer {
            if app.lang == Lang::De { "(Gemischt)" } else { "(Mixed)" }.to_string()
        } else {
            layers_list
                .iter()
                .find(|(id, _)| id == m_lid)
                .map(|(_, n)| n.clone())
                .unwrap_or_else(|| m_lid.to_string())
        };
        let mut move_layer_target: Option<String> = None;

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

                            egui::ComboBox::from_id_salt("sel_model_layer_combo")
                                .width(92.0)
                                .selected_text(&cur_layer_label)
                                .show_ui(ui, |ui| {
                                    for (lid, lname) in &layers_list {
                                        if ui.selectable_label(all_same_layer && layer_exists && lid == m_lid, lname).clicked() {
                                            move_layer_target = Some(lid.clone());
                                        }
                                    }
                                });
                            ui.separator();

                            let mut m_elev = model.elevation.unwrap_or(0.0);
                            let mut m_scale = model.scale.unwrap_or(1.0);
                            let mut m_rot = model.transform.rotation;
                            ui.label(RichText::new("Z:").size(11.0).color(p.text_muted));
                            if ui.add(egui::DragValue::new(&mut m_elev).speed(1.0).suffix(" mm")).changed() {
                                if let Some(ms) = &mut app.layout.models {
                                    for m in ms {
                                        if app.selected.contains(&m.id) {
                                            m.elevation = Some(m_elev);
                                        }
                                    }
                                }
                                app.mark_layout_dirty();
                            }
                            ui.label(RichText::new("Scale:").size(11.0).color(p.text_muted));
                            if ui.add(egui::DragValue::new(&mut m_scale).range(0.1..=20.0).speed(0.05).suffix("x")).changed() {
                                if let Some(ms) = &mut app.layout.models {
                                    for m in ms {
                                        if app.selected.contains(&m.id) {
                                            m.scale = Some(m_scale);
                                        }
                                    }
                                }
                                app.mark_layout_dirty();
                            }
                            ui.label(RichText::new("Rot:").size(11.0).color(p.text_muted));
                            if ui.add(egui::DragValue::new(&mut m_rot).range(-180.0..=360.0).speed(1.0).suffix(" deg")).changed() {
                                if let Some(ms) = &mut app.layout.models {
                                    for m in ms {
                                        if app.selected.contains(&m.id) {
                                            m.transform.rotation = railbench_core::geometry::norm_angle_deg(m_rot);
                                        }
                                    }
                                }
                                app.mark_layout_dirty();
                            }
                            ui.separator();

                            let sz = egui::vec2(26.0, 24.0);
                            if icon_button_sized(ui, Icon::RotateCcw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_ccw).clicked() {
                                app.rotate_selected(-app.rotation_step);
                            }
                            if icon_button_sized(ui, Icon::RotateCw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_cw).clicked() {
                                app.rotate_selected(app.rotation_step);
                            }
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Dup").strong().size(11.0).color(p.text_sub))
                                        .fill(p.bg_input)
                                        .stroke(Stroke::new(1.0_f32, p.border_main))
                                        .corner_radius(CornerRadius::same(8)),
                                )
                                .on_hover_text("Duplicate (Ctrl+D)")
                                .clicked()
                            {
                                app.duplicate_selected();
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
        if let Some(tid) = move_layer_target {
            app.move_selected_to_layer(&tid);
        }
        return;
    };

    let selected_pieces: Vec<_> = app
        .layout
        .pieces
        .iter()
        .filter(|pc| app.selected.contains(&pc.id))
        .cloned()
        .collect();

    let first_lid = sample.layer_id.as_deref().unwrap_or("layer-1");
    let all_same_layer = selected_pieces
        .iter()
        .all(|pc| pc.layer_id.as_deref().unwrap_or("layer-1") == first_lid);
    let layer_exists = layers_list.iter().any(|(id, _)| id == first_lid);
    let cur_layer_label = if !layer_exists {
        if app.lang == Lang::De { "(!) Ebene fehlt" } else { "(!) Missing layer" }.to_string()
    } else if !all_same_layer {
        if app.lang == Lang::De { "(Gemischt)" } else { "(Mixed)" }.to_string()
    } else {
        layers_list
            .iter()
            .find(|(id, _)| id == first_lid)
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| first_lid.to_string())
    };
    let mut move_layer_target: Option<String> = None;

    let any_turnout = selected_pieces.iter().any(|pc| {
        crate::catalog::find_entry(&app.catalogs, &pc.catalog_id)
            .map_or(false, |e| crate::is_turnout_kind(&e.kind))
    });
    let has_feeder = selected_pieces.iter().all(|pc| pc.power_feed == Some(true));
    let has_iso = selected_pieces.iter().all(|pc| pc.is_isolator == Some(true));
    let has_cat = selected_pieces.iter().all(|pc| pc.has_catenary == Some(true));
    let has_cat_feed = selected_pieces.iter().all(|pc| pc.catenary_power_feed == Some(true));
    let has_cat_iso = selected_pieces.iter().all(|pc| pc.catenary_isolator == Some(true));
    let has_sig = selected_pieces.iter().all(|pc| pc.has_signal == Some(true));
    let any_sig = selected_pieces.iter().any(|pc| pc.has_signal == Some(true));
    let sig_side = selected_pieces
        .iter()
        .find(|pc| pc.has_signal == Some(true))
        .and_then(|pc| pc.signal_side.as_deref())
        .unwrap_or("right");
    let sig_dir = selected_pieces
        .iter()
        .find(|pc| pc.has_signal == Some(true))
        .and_then(|pc| pc.signal_direction.as_deref())
        .unwrap_or("forward");
    let has_lantern = selected_pieces.iter().any(|pc| pc.has_lantern == Some(true));
    let has_decoder = selected_pieces.iter().any(|pc| pc.has_decoder == Some(true));
    let has_bridge = selected_pieces.iter().all(|pc| pc.has_bridge == Some(true));
    let has_tunnel = selected_pieces.iter().all(|pc| pc.has_tunnel == Some(true));
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

                        // Layer dropdown (Move selected to layer)
                        egui::ComboBox::from_id_salt("sel_bar_layer_combo")
                            .width(88.0)
                            .selected_text(&cur_layer_label)
                            .show_ui(ui, |ui| {
                                for (lid, lname) in &layers_list {
                                    if ui.selectable_label(all_same_layer && layer_exists && lid == first_lid, lname).clicked() {
                                        move_layer_target = Some(lid.clone());
                                    }
                                }
                            });
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

                            ui.label(RichText::new("W:").size(11.5).color(p.text_muted));
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

                        let toggle_pill = |ui: &mut egui::Ui, label: &str, active: bool, active_col: Color32, tip: &str| -> bool {
                            let fill = if active { active_col } else { p.bg_input };
                            let txt = if active { Color32::WHITE } else { p.text_sub };
                            ui.add(
                                egui::Button::new(RichText::new(label).strong().size(11.0).color(txt))
                                    .fill(fill)
                                    .stroke(Stroke::new(1.0_f32, if active { active_col } else { p.border_main }))
                                    .corner_radius(CornerRadius::same(9)),
                            )
                            .on_hover_text(tip)
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
                        if toggle_pill(ui, t.catenary, has_cat, p.primary_hover, "Catenary (O)") {
                            app.toggle_selected_catenary();
                        }
                        if toggle_pill(ui, "OL+", has_cat_feed, Color32::from_rgb(2, 132, 199), "Catenary Feeder (Shift+F)") {
                            app.toggle_selected_catenary_feeder();
                        }
                        let ol_iso_lbl = if sel_count == 1 && has_cat_iso {
                            let idx = sample.catenary_isolated_connector_index.unwrap_or(0) + 1;
                            let total = sample.connectors.len().max(2);
                            format!("OL/ {idx}/{total}")
                        } else {
                            "OL/".into()
                        };
                        if toggle_pill(ui, &ol_iso_lbl, has_cat_iso, Color32::from_rgb(220, 38, 38), "Catenary Isolator (Shift+I)") {
                            app.toggle_selected_catenary_isolator();
                        }
                        if toggle_pill(ui, "SIG", has_sig, Color32::from_rgb(22, 163, 74), "Signal (Shift+S)") {
                            app.toggle_selected_signal();
                        }
                        if any_sig {
                            let side_lbl = match sig_side {
                                "left" => "L",
                                "both" => "L+R",
                                _ => "R",
                            };
                            if toggle_pill(ui, side_lbl, true, Color32::from_rgb(15, 118, 110), "Cycle Signal Side") {
                                app.cycle_selected_signal_side();
                            }
                            let dir_lbl = match sig_dir {
                                "backward" => "<-",
                                "both" => "<->",
                                _ => "->",
                            };
                            if toggle_pill(ui, dir_lbl, true, Color32::from_rgb(15, 118, 110), "Cycle Signal Direction") {
                                app.cycle_selected_signal_direction();
                            }
                        }
                        if any_turnout {
                            let lantern_lbl = if app.lang == Lang::De { "Laterne" } else { "Lantern" };
                            if toggle_pill(ui, lantern_lbl, has_lantern, Color32::from_rgb(202, 138, 4), "Turnout Lantern (Shift+L)") {
                                app.toggle_selected_lantern();
                            }
                            if toggle_pill(ui, "DEC", has_decoder, Color32::from_rgb(2, 132, 199), "Turnout Decoder (Shift+D)") {
                                app.toggle_selected_decoder();
                            }
                        }
                        if toggle_pill(ui, t.bridge, has_bridge, p.primary_hover, "Bridge (B)") {
                            app.toggle_selected_bridge();
                        }
                        if toggle_pill(ui, t.tunnel, has_tunnel, p.primary_hover, "Tunnel (T)") {
                            app.toggle_selected_tunnel();
                        }

                        ui.separator();
                        if icon_button_sized(ui, Icon::RotateCcw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_ccw).clicked() {
                            app.rotate_selected(-app.rotation_step);
                        }
                        if icon_button_sized(ui, Icon::RotateCw, sz, false, p.bg_input, p.primary_hover, p.primary, p.text_main, t.rotate_cw).clicked() {
                            app.rotate_selected(app.rotation_step);
                        }
                        if toggle_pill(ui, "Dup", false, p.primary_hover, "Duplicate (Ctrl+D)") {
                            app.duplicate_selected();
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

    if let Some(tid) = move_layer_target {
        app.move_selected_to_layer(&tid);
    }
}

pub fn draw_dialogs(ctx: &egui::Context, app: &mut RailbenchApp) {
    let t = tr(app.lang);

    if app.show_power_dialog {
        let mut open = app.show_power_dialog;
        let is_de = app.lang == Lang::De;
        let sim_snapshot = app.power_sim_cache.as_ref().map(|(_, s)| s.clone());
        let mut param_changed = false;
        let mut focus_pt: Option<(f64, f64)> = None;
        let mut bulk_equip: Option<bool> = None;

        let mut total_turnouts = 0usize;
        let mut lantern_count = 0usize;
        let mut decoder_count = 0usize;
        let mut cat_pieces = 0usize;
        let mut cat_feeders = 0usize;
        let mut cat_isolators = 0usize;
        for pc in &app.layout.pieces {
            if crate::catalog::find_entry(&app.catalogs, &pc.catalog_id)
                .map_or(false, |e| crate::is_turnout_kind(&e.kind))
            {
                total_turnouts += 1;
                if pc.has_lantern == Some(true) {
                    lantern_count += 1;
                }
                if pc.has_decoder == Some(true) {
                    decoder_count += 1;
                }
            }
            if pc.has_catenary == Some(true) {
                cat_pieces += 1;
            }
            if pc.catenary_power_feed == Some(true) {
                cat_feeders += 1;
            }
            if pc.catenary_isolator == Some(true) {
                cat_isolators += 1;
            }
        }

        egui::Window::new(t.power_dialog)
            .open(&mut open)
            .resizable(false)
            .default_width(390.0)
            .show(ctx, |ui| {
                ui.checkbox(
                    &mut app.power_sim_enabled,
                    if is_de {
                        "Stromanalyse auf 2D-Gleisplan einblenden"
                    } else {
                        "Show Power Overlay on 2D Canvas"
                    },
                );
                ui.separator();

                let mut v = app.layout.feed_voltage.unwrap_or(16.0);
                let mut ma = app.layout.loco_current_ma.unwrap_or(450.0);
                let mut cnt = app.layout.loco_count.unwrap_or(1);

                ui.horizontal(|ui| {
                    ui.label(if is_de { "Vorauswahl:" } else { "Presets:" });
                    for (lbl, pv, pma) in [
                        ("12V (N)", 12.0, 100.0),
                        ("14V (DCC)", 14.0, 450.0),
                        ("16V (H0)", 16.0, 450.0),
                        ("18V (1)", 18.0, 800.0),
                    ] {
                        if ui.small_button(lbl).clicked() {
                            app.layout.feed_voltage = Some(pv);
                            app.layout.loco_current_ma = Some(pma);
                            app.power_sim_enabled = true;
                            v = pv;
                            ma = pma;
                            param_changed = true;
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(if is_de { "Fahrspannung:" } else { "Feed Voltage:" });
                    if ui
                        .add(egui::DragValue::new(&mut v).range(6.0..=24.0).speed(0.5).suffix(" V"))
                        .changed()
                    {
                        app.layout.feed_voltage = Some(v);
                        param_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Lokstrom:" } else { "Loco Current:" });
                    if ui
                        .add(egui::DragValue::new(&mut ma).range(50.0..=1500.0).speed(10.0).suffix(" mA"))
                        .changed()
                    {
                        app.layout.loco_current_ma = Some(ma);
                        param_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Aktive Loks:" } else { "Locos running:" });
                    if ui.add(egui::DragValue::new(&mut cnt).range(1..=20)).changed() {
                        app.layout.loco_count = Some(cnt);
                        param_changed = true;
                    }
                });

                ui.separator();
                ui.label(
                    RichText::new(if is_de {
                        "Oberleitung (2. Stromkreis)"
                    } else {
                        "Catenary Circuit"
                    })
                    .strong(),
                );
                let is_parallel = app.layout.catenary_circuit_mode.as_deref() == Some("parallel");
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Betriebsart:" } else { "Mode:" });
                    if ui
                        .selectable_label(!is_parallel, if is_de { "Getrennt" } else { "Separate" })
                        .clicked()
                    {
                        app.layout.catenary_circuit_mode = Some("separate".into());
                        param_changed = true;
                    }
                    if ui
                        .selectable_label(is_parallel, if is_de { "Parallel zum Gleis" } else { "Parallel to Track" })
                        .clicked()
                    {
                        app.layout.catenary_circuit_mode = Some("parallel".into());
                        param_changed = true;
                    }
                });
                if !is_parallel {
                    let mut cat_v = app.layout.catenary_feed_voltage.unwrap_or(v);
                    ui.horizontal(|ui| {
                        ui.label(if is_de { "Oberleitungsspannung:" } else { "Catenary Voltage:" });
                        if ui
                            .add(egui::DragValue::new(&mut cat_v).range(6.0..=24.0).speed(0.5).suffix(" V"))
                            .changed()
                        {
                            app.layout.catenary_feed_voltage = Some(cat_v);
                            param_changed = true;
                        }
                    });
                }
                ui.small(if is_de {
                    format!("Oberleitung: {cat_pieces} Gleise | {cat_feeders} Einspeiser | {cat_isolators} Trennstellen")
                } else {
                    format!("Catenary: {cat_pieces} tracks | {cat_feeders} feeders | {cat_isolators} isolators")
                });

                ui.separator();
                ui.label(
                    RichText::new(if is_de {
                        "Weichenlaternen & Digital-Decoder"
                    } else {
                        "Turnout Lanterns & Decoders"
                    })
                    .strong(),
                );
                let mut l_ma = app.layout.turnout_lantern_current_ma.unwrap_or(20.0);
                let mut d_ma = app.layout.turnout_decoder_current_ma.unwrap_or(15.0);
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Laterne:" } else { "Lantern:" });
                    if ui
                        .add(egui::DragValue::new(&mut l_ma).range(0.0..=100.0).speed(1.0).suffix(" mA"))
                        .changed()
                    {
                        app.layout.turnout_lantern_current_ma = Some(l_ma);
                        param_changed = true;
                    }
                    ui.label(if is_de { "Decoder:" } else { "Decoder:" });
                    if ui
                        .add(egui::DragValue::new(&mut d_ma).range(0.0..=100.0).speed(1.0).suffix(" mA"))
                        .changed()
                    {
                        app.layout.turnout_decoder_current_ma = Some(d_ma);
                        param_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.small(if is_de {
                        format!("Weichen: {total_turnouts} | Laternen: {lantern_count} | Decoder: {decoder_count}")
                    } else {
                        format!("Turnouts: {total_turnouts} | Lanterns: {lantern_count} | Decoders: {decoder_count}")
                    });
                });
                ui.horizontal(|ui| {
                    if ui
                        .button(if is_de { "Alle Weichen ausstatten" } else { "Equip All Turnouts" })
                        .clicked()
                    {
                        bulk_equip = Some(true);
                    }
                    if ui
                        .button(if is_de { "Alle entfernen" } else { "Clear All" })
                        .clicked()
                    {
                        bulk_equip = Some(false);
                    }
                });

                if let Some(sim) = sim_snapshot {
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "System: {}  |  Load: {:.2} A",
                            if sim.is_two_wire { "2-Wire (DCC/DC)" } else { "3-Wire AC" },
                            sim.total_current_a
                        ))
                        .strong(),
                    );
                    ui.label(format!(
                        "Feeders: {}  |  Isolators: {}  |  Min Voltage: {:.2} V (Drop {:.1}%)",
                        sim.feeder_count, sim.isolator_count, sim.min_voltage, sim.max_drop_pct
                    ));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("OK {}", sim.optimal_count)).color(Color32::from_rgb(34, 197, 94)));
                        ui.label(RichText::new(format!("Warn {}", sim.warning_count)).color(Color32::from_rgb(245, 158, 11)));
                        ui.label(RichText::new(format!("Crit {}", sim.critical_count)).color(Color32::from_rgb(239, 68, 68)));
                        ui.label(RichText::new(format!("Dead {}", sim.disconnected_count)).color(Color32::from_rgb(148, 163, 184)));
                    });
                    if sim.has_short_circuit {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("SHORT CIRCUIT ({} point(s))", sim.short_circuit_points.len()))
                                    .strong()
                                    .color(Color32::from_rgb(239, 68, 68)),
                            );
                            if let Some(pt) = sim.short_circuit_points.first() {
                                if ui.small_button("Focus").clicked() {
                                    focus_pt = Some((pt.x, pt.y));
                                }
                            }
                        });
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new(&sim.recommendation).size(11.5));
                }
            });
        app.show_power_dialog = open;
        if let Some(equip) = bulk_equip {
            app.bulk_equip_turnouts(equip);
        } else if param_changed {
            app.mark_layout_dirty();
        }
        if let Some((x, y)) = focus_pt {
            app.focus_world_point(x, y);
        }
    }

    if app.show_height_dialog {
        let mut open = app.show_height_dialog;
        let nem_h = min_clearance_for_scale(&app.layout.scale);
        let scale_label = app.layout.scale.clone();
        egui::Window::new(t.height_dialog)
            .open(&mut open)
            .resizable(false)
            .default_width(350.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut app.height_tab, 0, "Flat Height");
                    ui.selectable_value(&mut app.height_tab, 1, "Auto-Ramp");
                    ui.selectable_value(&mut app.height_tab, 2, "Ramp / Helix Gen");
                });
                ui.separator();

                match app.height_tab {
                    0 => {
                        ui.horizontal(|ui| {
                            ui.label("Elevation:");
                            ui.add(egui::DragValue::new(&mut app.batch_elevation_mm).speed(2.0).suffix(" mm"));
                            if ui.small_button(format!("NEM 102 ({nem_h:.0} mm {scale_label})")).clicked() {
                                app.batch_elevation_mm = nem_h;
                            }
                        });
                        ui.add_space(4.0);
                        if ui
                            .add_enabled(!app.selected.is_empty(), egui::Button::new("Apply flat height to selected"))
                            .clicked()
                        {
                            app.push_undo();
                            let h = app.batch_elevation_mm;
                            for piece in &mut app.layout.pieces {
                                if app.selected.contains(&piece.id) {
                                    piece.elevation = Some(Elevation { start: h, end: h });
                                }
                            }
                            app.mark_layout_dirty();
                        }
                    }
                    1 => {
                        ui.horizontal(|ui| {
                            ui.label("Start:");
                            ui.add(egui::DragValue::new(&mut app.ramp_start_h).speed(2.0).suffix(" mm"));
                            ui.label("End:");
                            ui.add(egui::DragValue::new(&mut app.ramp_end_h).speed(2.0).suffix(" mm"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Profile:");
                            ui.radio_value(&mut app.ramp_profile, RampProfile::Linear, "Linear");
                            ui.radio_value(&mut app.ramp_profile, RampProfile::Eased, "Eased (S-Curve)");
                        });
                        if let Some(sum) = preview_auto_ramp(
                            &app.layout,
                            &app.catalogs,
                            &app.selected,
                            app.ramp_start_h,
                            app.ramp_end_h,
                            app.ramp_profile,
                        ) {
                            let grade_col = if sum.max_gradient_pct <= 3.0 {
                                Color32::from_rgb(34, 197, 94)
                            } else if sum.max_gradient_pct <= 4.0 {
                                Color32::from_rgb(245, 158, 11)
                            } else {
                                Color32::from_rgb(239, 68, 68)
                            };
                            ui.label(format!(
                                "{} pieces  •  Length: {:.1} mm  •  Rise: {:.1} mm",
                                sum.piece_count, sum.total_length_mm, sum.total_rise_mm
                            ));
                            ui.label(
                                RichText::new(format!("Peak grade: {:.1}% (avg {:.1}%)", sum.max_gradient_pct, sum.avg_gradient_pct))
                                    .strong()
                                    .color(grade_col),
                            );
                        } else {
                            ui.weak("Select a chain of track pieces to preview ramp gradient.");
                        }
                        ui.add_space(4.0);
                        if ui
                            .add_enabled(!app.selected.is_empty(), egui::Button::new("Apply Auto-Ramp"))
                            .clicked()
                        {
                            app.push_undo();
                            match apply_auto_ramp(
                                &mut app.layout,
                                &app.catalogs,
                                &app.selected,
                                app.ramp_start_h,
                                app.ramp_end_h,
                                app.ramp_profile,
                            ) {
                                Ok(sum) => {
                                    app.mark_layout_dirty();
                                    app.status_message = Some(format!(
                                        "Applied ramp over {} pieces ({:.1}% peak grade)",
                                        sum.piece_count, sum.max_gradient_pct
                                    ));
                                }
                                Err(e) => app.status_message = Some(e),
                            }
                        }
                    }
                    _ => {
                        let single_sel = if app.selected.len() == 1 {
                            app.selected.iter().next().cloned()
                        } else {
                            None
                        };
                        if single_sel.is_none() {
                            ui.label(
                                RichText::new("Select 1 anchor track with an open connector to generate from.")
                                    .color(Color32::from_rgb(245, 158, 11)),
                            );
                        }
                        ui.horizontal(|ui| {
                            ui.label("Shape:");
                            ui.radio_value(&mut app.gen_ramp_shape, RampShape::Straight, "Straight");
                            ui.radio_value(&mut app.gen_ramp_shape, RampShape::Curved, "Curved");
                            ui.radio_value(&mut app.gen_ramp_shape, RampShape::Helix, "Helix");
                        });
                        if app.gen_ramp_shape != RampShape::Straight {
                            ui.horizontal(|ui| {
                                ui.label("Direction:");
                                ui.radio_value(&mut app.gen_curve_left, true, "Left");
                                ui.radio_value(&mut app.gen_curve_left, false, "Right");
                            });
                            if let Some(cat) = app.catalogs.get(app.selected_catalog_idx) {
                                let curves = available_curves_for_catalog(cat);
                                if !curves.is_empty() {
                                    let sel_label = app
                                        .gen_curve_id
                                        .as_deref()
                                        .and_then(|cid| curves.iter().find(|c| c.id == cid))
                                        .map(|c| format!("{} (R{:.0})", c.id, c.radius.unwrap_or(0.0)))
                                        .unwrap_or_else(|| "Auto standard curve".into());
                                    egui::ComboBox::from_id_salt("gen_curve_combo")
                                        .selected_text(sel_label)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut app.gen_curve_id, None, "Auto standard curve");
                                            for c in &curves {
                                                let lbl = format!(
                                                    "{} • R{:.0} mm ({:.1}°)",
                                                    c.id,
                                                    c.radius.unwrap_or(0.0),
                                                    c.angle.unwrap_or(30.0)
                                                );
                                                ui.selectable_value(&mut app.gen_curve_id, Some(c.id.clone()), lbl);
                                            }
                                        });
                                }
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label("Target H:");
                            ui.add(egui::DragValue::new(&mut app.gen_target_h).speed(2.0).suffix(" mm"));
                            if ui.small_button(format!("+{nem_h:.0}")).clicked() {
                                app.gen_target_h = nem_h;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Max Grade:");
                            ui.add(egui::DragValue::new(&mut app.gen_max_grade).range(0.5..=8.0).speed(0.1).suffix(" %"));
                            if app.gen_ramp_shape == RampShape::Helix {
                                ui.label("Turns:");
                                ui.add(egui::DragValue::new(&mut app.gen_helix_turns).range(0.5..=6.0).speed(0.25));
                            }
                        });
                        ui.add_space(4.0);
                        if ui
                            .add_enabled(single_sel.is_some(), egui::Button::new("Generate Ramp / Helix"))
                            .clicked()
                        {
                            if let Some(anchor_id) = single_sel {
                                app.push_undo();
                                match generate_ramp_or_helix(
                                    &mut app.layout,
                                    &app.catalogs,
                                    app.selected_catalog_idx,
                                    &mut app.next_piece_seq,
                                    &anchor_id,
                                    app.gen_ramp_shape,
                                    app.gen_curve_left,
                                    app.gen_target_h,
                                    app.gen_max_grade,
                                    app.gen_helix_turns,
                                    app.gen_curve_id.as_deref(),
                                ) {
                                    Ok((new_ids, sum)) => {
                                        app.selected.clear();
                                        for id in new_ids {
                                            app.selected.insert(id);
                                        }
                                        app.mark_layout_dirty();
                                        app.status_message = Some(format!(
                                            "Generated {} pieces ({:.1}% grade, {:.1} mm rise)",
                                            sum.piece_count, sum.avg_gradient_pct, sum.total_rise_mm
                                        ));
                                    }
                                    Err(e) => app.status_message = Some(e),
                                }
                            }
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

    if app.show_welcome_dialog {
        draw_welcome_dialog(ctx, app);
    }

    if app.show_demo_dialog {
        draw_demo_dialog(ctx, app);
    }

    if app.show_settings_dialog {
        draw_settings_dialog(ctx, app);
    }

    if app.show_export_dialog {
        draw_export_dialog(ctx, app);
    }

    if app.show_help_dialog {
        draw_help_dialog(ctx, app);
    }
}

fn draw_welcome_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
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

fn draw_demo_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
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

fn draw_settings_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
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
                dirty_settings |= ui.checkbox(&mut app.show_right_sidebar, if is_de { "Schwebendes Ebenen-Menue sichtbar" } else { "Floating Layers Menu Visible" }).changed();
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
                    dirty_settings |= ui.radio_value(&mut app.viewer3d_env, crate::viewer3d::Viewer3DEnv::Hall, if is_de { "Halle" } else { "Hall" }).changed();
                    dirty_settings |= ui.radio_value(&mut app.viewer3d_env, crate::viewer3d::Viewer3DEnv::Landscape, if is_de { "Landschaft" } else { "Landscape" }).changed();
                    dirty_settings |= ui.radio_value(&mut app.viewer3d_env, crate::viewer3d::Viewer3DEnv::Minimal, "Minimal").changed();
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

fn draw_export_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
    let is_de = app.lang == Lang::De;
    let p = palette(app.dark_mode);
    let mut open = app.show_export_dialog;
    let mut trigger: Option<u8> = None;

    egui::Window::new(if is_de {
        "Bauplan-Export & Drucken"
    } else {
        "Blueprint Export & Print"
    })
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(430.0)
    .show(ctx, |ui| {
        Frame::new()
            .fill(p.bg_card)
            .corner_radius(CornerRadius::same(7))
            .inner_margin(Margin::same(8))
            .stroke(Stroke::new(1.0_f32, p.border_main))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(format!("{} (Spur {})", app.layout.name, app.layout.scale)).strong());
                ui.small(if is_de {
                    format!(
                        "{} Gleise | {} Modelle | {} Ebenen",
                        app.layout.pieces.len(),
                        app.layout.models.as_ref().map_or(0, Vec::len),
                        app.layout.layers.as_ref().map_or(1, Vec::len)
                    )
                } else {
                    format!(
                        "{} pieces | {} models | {} layers",
                        app.layout.pieces.len(),
                        app.layout.models.as_ref().map_or(0, Vec::len),
                        app.layout.layers.as_ref().map_or(1, Vec::len)
                    )
                });
            });
        ui.add_space(8.0);

        ui.label(RichText::new(if is_de { "Seitenformat (PDF)" } else { "Page Layout (PDF)" }).strong());
        ui.horizontal(|ui| {
            ui.radio_value(&mut app.export_opts.landscape, true, if is_de { "A4 Querformat" } else { "A4 Landscape" });
            ui.radio_value(&mut app.export_opts.landscape, false, if is_de { "A4 Hochformat" } else { "A4 Portrait" });
        });
        ui.checkbox(
            &mut app.export_opts.include_overview,
            if is_de { "Gesamtuebersicht einschliessen" } else { "Include Full Overview Page" },
        );
        ui.checkbox(
            &mut app.export_opts.include_layer_pages,
            if is_de { "Separate Bauplaene je Ebene (PDF)" } else { "Include Per-Layer Blueprint Pages (PDF)" },
        );
        if app.export_opts.include_layer_pages {
            if let Some(layers) = &app.layout.layers {
                ui.indent("export_layer_sel", |ui| {
                    ui.small(if is_de {
                        "Ebenenauswahl (leer = alle sichtbaren Ebenen):"
                    } else {
                        "Layer filter (empty = all visible layers):"
                    });
                    ui.horizontal_wrapped(|ui| {
                        for lay in layers {
                            let mut checked = app.export_opts.selected_layer_ids.contains(&lay.id);
                            if ui.checkbox(&mut checked, &lay.name).changed() {
                                if checked {
                                    app.export_opts.selected_layer_ids.push(lay.id.clone());
                                } else {
                                    app.export_opts.selected_layer_ids.retain(|id| id != &lay.id);
                                }
                            }
                        }
                    });
                });
            }
        }
        ui.checkbox(
            &mut app.export_opts.include_bom_page,
            if is_de { "Stueckliste am Ende anhaengen (PDF)" } else { "Include Bill of Materials Page (PDF)" },
        );

        ui.add_space(6.0);
        ui.label(RichText::new(if is_de { "Darstellung (PDF & SVG)" } else { "Rendering Options (PDF & SVG)" }).strong());
        ui.checkbox(
            &mut app.export_opts.show_baseboards,
            if is_de { "Grundplatten / Tische zeichnen" } else { "Draw Baseboards / Tables" },
        );
        ui.checkbox(
            &mut app.export_opts.show_labels,
            if is_de { "Gleis-Artikelnummern beschriften" } else { "Show Track Part Numbers" },
        );
        ui.checkbox(
            &mut app.export_opts.ghost_other_layers,
            if is_de { "Andere Ebenen ausgegraut hinterlegen" } else { "Ghost Other Layers on Layer Pages" },
        );
        ui.checkbox(
            &mut app.export_opts.monochrome,
            if is_de { "Schwarz-Weiss-Bauplan (Monochrom)" } else { "Black & White Blueprint (Monochrome)" },
        );

        ui.add_space(10.0);
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("PDF Blueprint...").strong().color(Color32::WHITE))
                        .fill(p.primary_hover),
                )
                .clicked()
            {
                trigger = Some(1);
            }
            if ui.button("SVG (1:1 mm)...").clicked() {
                trigger = Some(2);
            }
            if ui.button("XML (.xml)...").clicked() {
                trigger = Some(3);
            }
            if ui.button("BOM (CSV)...").clicked() {
                trigger = Some(4);
            }
        });
    });

    app.show_export_dialog = open;
    match trigger {
        Some(1) => app.export_pdf_dialog(),
        Some(2) => app.export_svg_dialog(),
        Some(3) => app.export_xml_dialog(),
        Some(4) => app.export_bom_csv_dialog(),
        _ => {}
    }
}

fn draw_help_dialog(ctx: &egui::Context, app: &mut RailbenchApp) {
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
                    "- Auto-Snap: Gleise snappen an Gleise, Strassen separat an Strassen.\n- '+' Schnell-Anfuegen: Klick auf den blauen '+'-Button eines offenen Verbinders verlaengert direkt mit dem zuletzt gewaehlten Stueck.\n- Flexgleise: Ausgewaehlte Flexgleise bieten 3 blaue Biegepunkte (25%, 50%, 75% Spline) sowie einen orangenen Endpunkt zum Kuerzen und Schwenken."
                } else {
                    "- Auto-Snap: Tracks snap cleanly to tracks, roads snap separately to roads.\n- '+' Quick-Add: Click the cyan '+' badge on any open connector to extend immediately.\n- Flextracks: Selected flex pieces reveal 3 interior spline bend points (25%, 50%, 75%) plus an amber length/swing endpoint handle."
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
                    "- 16 integrierte 3D-Gebaeude & Baeume mit automatischer Massstabsanpassung sowie Lokschuppen-Torsnapping.\n- Eigene .glb / .gltf 3D-Modelle per Datei-Dialog oder Drag & Drop importieren.\n- Berg-Pinsel & Boden-Platten modellieren direkt 3D-Gelaende.\n- 360-Grad Orbit-Kamera, CAD-ViewCube, Tag/Nacht-Zyklus sowie Halle/Landschaft/Minimal-Umgebungen."
                } else {
                    "- 16 scale-aware builtin 3D structures, plus roundhouse/shed door snapping for tracks.\n- Import custom .glb / .gltf models via File Dialog or Drag & Drop.\n- Paint mountains interactively with the Mountain Brush.\n- Full 360/180 orbit camera, CAD ViewCube, Day/Night lighting, and Hall/Landscape/Minimal scenes."
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
        wood_plate_mode: Some("table".into()),
        wood_plate_thickness_mm: Some(12.0),
        wood_plate_style: None,
        plates: None,
    }
}
