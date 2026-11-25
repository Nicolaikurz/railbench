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
    apply_auto_ramp_for_chain, available_curves_for_catalog, generate_ramp_or_helix,
    min_clearance_for_scale, preview_auto_ramp_for_chain, resolve_ramp_chain, RampProfile,
    RampShape,
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
                // RAILbench brand badge + CalVer tag
                ui.label(RichText::new("RAIL").strong().size(14.5).color(p.primary));
                ui.add_space(-6.0);
                ui.label(RichText::new("bench").strong().size(14.5).color(Color32::WHITE));
                ui.add_space(-2.0);
                ui.label(
                    RichText::new(crate::version::APP_VERSION_TAG)
                        .size(10.5)
                        .monospace()
                        .color(p.text_muted),
                );
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
                    let import_cat_lbl = if app.lang == Lang::De {
                        "Gleiskatalog importieren (.json)..."
                    } else {
                        "Import Track Catalog (.json)..."
                    };
                    if ui.button(import_cat_lbl).clicked() {
                        app.import_catalog_dialog();
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
                    let view3d_lbl = if is_de { "3D-Ansicht umschalten (3)" } else { "Toggle 3D View (3)" };
                    if ui.button(view3d_lbl).clicked() {
                        app.toggle_3d();
                        ui.close_menu();
                    }
                    ui.separator();
                    let mut view_changed = false;
                    view_changed |= ui.checkbox(&mut app.show_labels, format!("{} (N)", t.labels)).changed();
                    view_changed |= ui.checkbox(&mut app.show_connectors, t.connectors).changed();
                    view_changed |= ui
                        .checkbox(&mut app.show_rulers, if is_de { "Lineale & Raster (Shift+L)" } else { "Rulers & Grid (Shift+L)" })
                        .changed();
                    view_changed |= ui.checkbox(&mut app.snapping_enabled, format!("{} (S)", t.snapping)).changed();
                    view_changed |= ui
                        .checkbox(
                            &mut app.layer_protection,
                            if is_de {
                                "Ebenenschutz (Alt+L)"
                            } else {
                                "Layer Protection (Alt+L)"
                            },
                        )
                        .changed();
                    view_changed |= ui
                        .checkbox(&mut app.show_wood_plates, if is_de { "Holzplatten anzeigen" } else { "Show Wood Plates" })
                        .changed();
                    view_changed |= ui.checkbox(&mut app.show_right_sidebar, format!("{} (Shift+B)", t.layers_title)).changed();
                    ui.checkbox(&mut app.power_sim_enabled, format!("{} (P)", t.volt_btn));
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

                // Snap + Layer Protection + Measure + Rotate split group (monochrome icons)
                if tb_icon(ui, Icon::Magnet, app.snapping_enabled, t.snapping) {
                    app.snapping_enabled = !app.snapping_enabled;
                    app.save_current_settings();
                }
                let layer_prot_icon = if app.layer_protection {
                    Icon::Lock
                } else {
                    Icon::Unlock
                };
                let layer_prot_tip = if app.lang == Lang::De {
                    if app.layer_protection {
                        "Ebenenschutz aktiv: Nur Objekte der aktiven Ebene koennen bewegt werden (Klicken zum Deaktivieren)"
                    } else {
                        "Ebenenschutz deaktiviert: Alle sichtbaren Ebenen koennen bewegt werden (Klicken zum Aktivieren)"
                    }
                } else if app.layer_protection {
                    "Layer protection active: Only active layer objects can be moved (Click to disable)"
                } else {
                    "Layer protection disabled: Objects on any visible layer can be moved (Click to enable)"
                };
                if tb_icon(ui, layer_prot_icon, app.layer_protection, layer_prot_tip) {
                    app.layer_protection = !app.layer_protection;
                    app.save_current_settings();
                }
                if tb_icon(ui, Icon::Ruler, app.measure_mode, "Measure distance") {
                    app.measure_mode = !app.measure_mode;
                    if !app.measure_mode {
                        app.measure_points = None;
                    }
                }

                // CAD Planning Tools Dropdown (Gap Closer, Parallel Tracks, NEM Clearance, Reachability Heatmap)
                let is_de_cad = app.lang == Lang::De;
                let cad_lbl = if is_de_cad { "CAD-Tools ▾" } else { "CAD ▾" };
                ui.menu_button(cad_lbl, |ui| {
                    ui.set_min_width(240.0);
                    if ui
                        .button(if is_de_cad {
                            "Gleisluecke schliessen (Auto)"
                        } else {
                            "Close Track Gap (Auto)"
                        })
                        .on_hover_text(if is_de_cad {
                            "Verbindet zwei offene Gleisenden automatisch mit Katalog- oder Flexgleis"
                        } else {
                            "Bridges two open track connectors with exact catalog or flex pieces"
                        })
                        .clicked()
                    {
                        app.run_close_gap();
                        ui.close_menu();
                    }

                    ui.separator();
                    ui.strong(if is_de_cad {
                        "Parallelgleis-Generator"
                    } else {
                        "Parallel Track Generator"
                    });
                    ui.horizontal(|ui| {
                        ui.label(if is_de_cad { "Abstand:" } else { "Spacing:" });
                        ui.add(
                            egui::DragValue::new(&mut app.cad_tools.parallel_spacing_mm)
                                .range(20.0..=250.0)
                                .speed(0.5)
                                .suffix(" mm"),
                        );
                        if ui
                            .selectable_label(app.cad_tools.parallel_side < 0.0, if is_de_cad { "Links" } else { "L" })
                            .clicked()
                        {
                            app.cad_tools.parallel_side = -1.0;
                        }
                        if ui
                            .selectable_label(app.cad_tools.parallel_side >= 0.0, if is_de_cad { "Rechts" } else { "R" })
                            .clicked()
                        {
                            app.cad_tools.parallel_side = 1.0;
                        }
                    });
                    if ui
                        .button(if is_de_cad {
                            "Parallelgleis aus Auswahl erzeugen"
                        } else {
                            "Generate Parallel from Selection"
                        })
                        .clicked()
                    {
                        app.run_parallel_tracks();
                        ui.close_menu();
                    }

                    ui.separator();
                    if ui
                        .checkbox(
                            &mut app.cad_tools.nem_check_enabled,
                            if is_de_cad {
                                "Lichtraumprofil NEM 102/103 pruefen"
                            } else {
                                "Check NEM 102/103 Clearances"
                            },
                        )
                        .changed()
                    {
                        app.cad_tools.nem_cache_version = 0;
                    }
                    ui.checkbox(
                        &mut app.cad_tools.reachability_enabled,
                        if is_de_cad {
                            "Greifweite-Heatmap anzeigen"
                        } else {
                            "Show Reachability Heatmap"
                        },
                    );
                    if app.cad_tools.reachability_enabled {
                        ui.horizontal(|ui| {
                            ui.label(if is_de_cad { "Armreichweite:" } else { "Arm reach:" });
                            ui.add(
                                egui::DragValue::new(&mut app.cad_tools.max_reach_mm)
                                    .range(450.0..=1200.0)
                                    .speed(10.0)
                                    .suffix(" mm"),
                            );
                        });
                    }
                });

                if app.cad_tools.nem_check_enabled && !app.cad_tools.nem_violations.is_empty() {
                    let cnt = app.cad_tools.nem_violations.len();
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(format!("NEM ({cnt})"))
                                    .strong()
                                    .size(11.0)
                                    .color(Color32::BLACK),
                            )
                            .fill(Color32::from_rgb(251, 191, 36))
                            .corner_radius(CornerRadius::same(6)),
                        )
                        .on_hover_text(if is_de_cad {
                            "NEM-Lichtraumkonflikt fokussieren"
                        } else {
                            "Cycle and focus NEM clearance violation"
                        })
                        .clicked()
                    {
                        let idx = app.cad_tools.nem_focus_idx % cnt;
                        let pos = app.cad_tools.nem_violations[idx].pos;
                        app.cad_tools.nem_focus_idx = (idx + 1) % cnt;
                        app.focus_world_point(pos.x, pos.y);
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

                    let is_de = app.lang == Lang::De;

                    // Red 2-Wire Short-Circuit alert pill (ALWAYS visible on 2-wire shorts, even when Power Overlay is off!)
                    let first_sc = app
                        .power_sim_cache
                        .as_ref()
                        .filter(|(_, s)| s.has_short_circuit && (s.is_two_wire || app.power_sim_enabled))
                        .map(|(_, s)| {
                            (
                                s.is_two_wire,
                                s.short_circuit_points.len(),
                                s.short_circuit_points.first().map(|pt| (pt.x, pt.y)),
                            )
                        });
                    if let Some((is_two_wire, cnt, maybe_pt)) = first_sc {
                        let label = if is_two_wire {
                            if is_de {
                                if cnt > 0 { format!("⚠ 2-LEITER KURZSCHLUSS ({cnt})") } else { "⚠ 2-LEITER KURZSCHLUSS".into() }
                            } else if cnt > 0 {
                                format!("⚠ 2-WIRE SHORT ({cnt})")
                            } else {
                                "⚠ 2-WIRE SHORT".into()
                            }
                        } else if cnt > 0 {
                            format!("⚡ SHORT ({cnt})")
                        } else {
                            "⚡ SHORT".into()
                        };
                        if ui
                            .add(
                                egui::Button::new(RichText::new(label).strong().size(11.0).color(Color32::WHITE))
                                    .fill(Color32::from_rgb(220, 38, 38))
                                    .corner_radius(CornerRadius::same(6)),
                            )
                            .on_hover_text(if is_de {
                                "2-Leiter Kurzschluss! Klicken zum Fokussieren — Trennstellen (Isolator) setzen."
                            } else {
                                "2-Wire short circuit detected! Click to focus — place Isolator tracks to break the loop."
                            })
                            .clicked()
                        {
                            app.power_sim_enabled = true;
                            if let Some((x, y)) = maybe_pt {
                                app.focus_world_point(x, y);
                            }
                        }
                    }

                // Unified Drive, Cab View & Signal Box Window (for 2nd Monitor)
                let drive_lbl = if is_de { "🚂 Fahren & Stellwerk" } else { "🚂 Drive & Signal Box" };
                let drive_active = app.show_cab_window || app.vehicle_sim.running;
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(drive_lbl)
                                .size(11.0)
                                .strong()
                                .color(if drive_active { Color32::WHITE } else { p.text_main }),
                        )
                        .fill(if app.vehicle_sim.running {
                            Color32::from_rgb(22, 163, 74)
                        } else if app.show_cab_window {
                            Color32::from_rgb(2, 132, 199)
                        } else {
                            p.bg_input
                        })
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .on_hover_text(if is_de {
                        "Fahrbetrieb, Fuehrerstand & Gleisbildstellwerk in einem Fenster (fuer 2. Monitor)"
                    } else {
                        "Open Drive, 3D Cab View & Signal Box (Stellwerk) in one window (for a 2nd monitor)"
                    })
                    .clicked()
                {
                    app.show_cab_window = !app.show_cab_window;
                }

                // 3D Toggle + Separate 3D Train Mode
                if tb_icon(ui, Icon::Box3d, app.is_3d_mode, "3D View (3)") {
                    app.toggle_3d();
                }
                let tr_active = app.is_3d_mode && app.train_3d.active;
                let tr_btn_lbl = if app.lang == Lang::De { "Zuege 3D" } else { "Trains 3D" };
                if pill_btn(ui, tr_btn_lbl, tr_active)
                    .on_hover_text(if app.lang == Lang::De {
                        "Separater 3D Zug-Modus (BR 243, BR 96 Mallet, V 60)"
                    } else {
                        "Separate 3D Train Mode (BR 243, BR 96 Mallet, V 60)"
                    })
                    .clicked()
                {
                    if tr_active {
                        app.train_3d.active = false;
                    } else {
                        if !app.is_3d_mode {
                            app.toggle_3d();
                        }
                        app.train_3d.enter_mode(&app.layout, &app.catalogs, &mut app.camera_3d);
                    }
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
            painter.circle_filled(c, 6.8, Color32::from_rgb(30, 41, 59));
            painter.circle_stroke(c, 9.5, Stroke::new(1.5_f32, Color32::from_rgb(148, 163, 184)));
            painter.circle_stroke(c, 5.4, Stroke::new(1.0_f32, Color32::from_rgb(100, 116, 139)));
            for i in 0..6 {
                let a = (i as f32 / 6.0) * std::f32::consts::TAU;
                let (s, cs) = a.sin_cos();
                painter.line_segment(
                    [egui::pos2(c.x + cs * 6.8, c.y + s * 6.8), egui::pos2(c.x + cs * 9.5, c.y + s * 9.5)],
                    Stroke::new(1.1_f32, accent),
                );
            }
            painter.line_segment([egui::pos2(c.x - 6.5, c.y), egui::pos2(c.x + 6.5, c.y)], stroke);
        }
        "transfer-table" => {
            let outer = egui::Rect::from_min_max(egui::pos2(l + 1.0, t + 1.0), egui::pos2(r - 1.0, b - 1.0));
            let pit = egui::Rect::from_min_max(egui::pos2(l + 4.0, t + 2.5), egui::pos2(r - 4.0, b - 2.5));
            painter.rect_stroke(
                outer,
                1.5,
                Stroke::new(1.3_f32, Color32::from_rgb(148, 163, 184)),
                egui::StrokeKind::Inside,
            );
            painter.rect_filled(pit, 1.0, Color32::from_rgb(30, 41, 59));
            for fx in [0.22_f32, 0.50, 0.78] {
                let tx = pit.min.x + pit.width() * fx;
                painter.line_segment(
                    [egui::pos2(tx, pit.min.y + 1.0), egui::pos2(tx, pit.max.y - 1.0)],
                    Stroke::new(1.0_f32, Color32::from_rgb(100, 116, 139)),
                );
            }
            for fy in [0.22_f32, 0.50, 0.78] {
                let ty = outer.min.y + outer.height() * fy;
                painter.line_segment([egui::pos2(outer.min.x, ty), egui::pos2(pit.min.x, ty)], Stroke::new(1.1_f32, accent));
                painter.line_segment([egui::pos2(pit.max.x, ty), egui::pos2(outer.max.x, ty)], Stroke::new(1.1_f32, accent));
            }
            painter.line_segment([egui::pos2(pit.min.x + 0.5, c.y), egui::pos2(pit.max.x - 0.5, c.y)], stroke);
        }
        "feeder" => {
            painter.line_segment([egui::pos2(l, c.y + 3.0), egui::pos2(r, c.y + 3.0)], stroke);
            painter.line_segment([egui::pos2(c.x + 2.0, t + 1.0), egui::pos2(c.x - 2.0, c.y)], stroke);
            painter.line_segment([egui::pos2(c.x - 2.0, c.y), egui::pos2(c.x + 2.0, c.y)], stroke);
            painter.line_segment([egui::pos2(c.x + 2.0, c.y), egui::pos2(c.x - 1.0, c.y + 4.0)], stroke);
        }
        "uncoupler" => {
            painter.line_segment([egui::pos2(l, c.y), egui::pos2(r, c.y)], stroke);
            painter.rect_filled(
                egui::Rect::from_center_size(c, egui::vec2(8.0, 5.0)),
                1.5,
                accent,
            );
        }
        _ => {
            painter.line_segment([egui::pos2(l, c.y), egui::pos2(r, c.y)], stroke);
            let tie_s = Stroke::new(1.3_f32, accent);
            for tx in [l + 4.0, c.x, r - 4.0] {
                painter.line_segment([egui::pos2(tx, c.y - 3.5), egui::pos2(tx, c.y + 3.5)], tie_s);
            }
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

mod left_tabs;
#[allow(unused_imports)]
use left_tabs::*;
#[allow(unused_imports)]
pub use left_tabs::{draw_left_sidebar};
mod terrain_tab;
#[allow(unused_imports)]
use terrain_tab::*;
mod board_bom_tabs;
#[allow(unused_imports)]
use board_bom_tabs::*;
mod right_and_selection;
#[allow(unused_imports)]
use right_and_selection::*;
#[allow(unused_imports)]
pub use right_and_selection::{draw_right_sidebar, draw_floating_selection_bar};
mod dialogs;
#[allow(unused_imports)]
use dialogs::*;
#[allow(unused_imports)]
pub use dialogs::{draw_dialogs};
mod extra_dialogs;
#[allow(unused_imports)]
use extra_dialogs::*;
mod export_dialog;
#[allow(unused_imports)]
use export_dialog::*;
mod cab_window;
pub use cab_window::draw_cab_view_window;
mod stellwerk_panel;
