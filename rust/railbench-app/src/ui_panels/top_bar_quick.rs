use super::*;

pub(super) fn draw_quick_toolbar(ctx: &egui::Context, app: &mut RailbenchApp) {
    let t = tr(app.lang);
    let p = palette(app.dark_mode);
    // =========================================================================
    //  LINE 2: Actions Row (Sidebar Register Tabs left | Tools center | DE/Collab/Theme/Layers right)
    //  Matches src/ui/toolbar.ts toolbar-actions-row
    // =========================================================================
    let qt_resp = egui::TopBottomPanel::top("quick_toolbar")
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(p.bg_toolbar)
                .inner_margin(Margin::symmetric(10, 5))
                .stroke(Stroke::NONE),
        )
        .show(ctx, |ui| {
            style_toolbar_ui(ui, p);
            // Every button, menu button, combo box and toggle in this row gets the same height.
            ui.spacing_mut().interact_size.y = TB_H;
            ui.spacing_mut().button_padding = egui::vec2(8.0, 3.0);
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
                            .min_size(egui::vec2(34.0, TB_H))
                            .corner_radius(CornerRadius::same(6)),
                    )
                };

                // Sidebar Register Tabs (aligned above left sidebar, matching #toolbar-sidebar-tabs)
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.spacing_mut().button_padding = egui::vec2(6.0, 3.0);
                let mut tabs_right_x = ui.cursor().left();
                for (tab, lbl, de_lbl, en_lbl) in [
                    (SidebarTab::Tracks, t.tab_tracks, DE.tab_tracks, EN.tab_tracks),
                    (SidebarTab::Roads, t.tab_roads, DE.tab_roads, EN.tab_roads),
                    (SidebarTab::Models, t.tab_models, DE.tab_models, EN.tab_models),
                    (SidebarTab::Terrain, t.tab_terrain, DE.tab_terrain, EN.tab_terrain),
                    (SidebarTab::Board, t.tab_board, DE.tab_board, EN.tab_board),
                    (SidebarTab::Bom, t.tab_bom, DE.tab_bom, EN.tab_bom),
                ] {
                    let active = app.sidebar_tab == tab;
                    let fill = if active { p.primary_hover } else { Color32::TRANSPARENT };
                    let txt_col = if active { Color32::WHITE } else { Color32::from_rgb(148, 163, 184) };
                    let tab_w = stable_text_width(ui, de_lbl, en_lbl, Some(11.5), true)
                        + ui.spacing().button_padding.x * 2.0;
                    let resp = ui.add(
                        egui::Button::new(RichText::new(lbl).strong().size(11.5).color(txt_col))
                            .fill(fill)
                            .stroke(if active {
                                Stroke::new(1.0_f32, p.primary)
                            } else {
                                Stroke::NONE
                            })
                            .min_size(egui::vec2(tab_w, TB_H))
                            .corner_radius(CornerRadius::same(5)),
                    );
                    tabs_right_x = resp.rect.right();
                    if resp.clicked() {
                        app.sidebar_tab = tab;
                    }
                }
                ui.spacing_mut().button_padding = egui::vec2(8.0, 3.0);
                ui.spacing_mut().item_spacing.x = 6.0;
                let required_sidebar_w = (tabs_right_x + 10.0).ceil().clamp(260.0, 520.0);
                ctx.data_mut(|d| {
                    d.insert_temp(egui::Id::new(LEFT_SIDEBAR_MIN_W_KEY), required_sidebar_w);
                });
                let cur_sidebar_w = ctx
                    .data(|d| d.get_temp::<f32>(egui::Id::new(LEFT_SIDEBAR_ACTUAL_W_KEY)))
                    .or(app.app_settings.left_sidebar_width)
                    .unwrap_or(required_sidebar_w)
                    .clamp(required_sidebar_w, 520.0);
                // egui::Ui::separator() allocates 6px wide and paints its vertical line at start_x + 3.0.
                let align_gap = (cur_sidebar_w - 3.0 - ui.cursor().left()).max(0.0);
                if align_gap > 0.0 {
                    ui.add_space(align_gap);
                }
                ui.separator();

                let tb_icon = |ui: &mut egui::Ui, icon: Icon, active: bool, tip: &str| -> bool {
                    icon_button_sized(
                        ui,
                        icon,
                        egui::vec2(30.0, TB_H),
                        active,
                        p.toolbar_btn,
                        p.primary_hover,
                        p.primary,
                        Color32::WHITE,
                        tip,
                    )
                    .clicked()
                };

                // Quick file actions (monochrome icons)
                if tb_icon(ui, Icon::FilePlus, false, t.new_layout) { app.request_new_layout(); }
                if tb_icon(ui, Icon::FolderOpen, false, t.open_file) { app.request_open_layout_dialog(); }
                if tb_icon(ui, Icon::Save, false, t.save_file) { app.save_layout_dialog(); }
                ui.separator();

                // Undo / Redo fused split group ([ ↶ | ↷ ])
                let can_undo = !app.undo_stack.is_empty();
                let can_redo = !app.redo_stack.is_empty();
                let undo_tip = format_undo_redo_tooltip(app.lang, false, app.undo_stack.len());
                let redo_tip = format_undo_redo_tooltip(app.lang, true, app.redo_stack.len());
                let mut trigger_undo = false;
                let mut trigger_redo = false;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let disabled_border = Color32::from_rgba_unmultiplied(148, 163, 184, 28);
                    let enabled_border = Color32::from_rgba_unmultiplied(148, 163, 184, 38);
                    let disabled_icon = Color32::from_rgb(138, 152, 172);
                    let split_div = Color32::from_rgba_unmultiplied(148, 163, 184, 52);

                    let segments = [
                        (
                            Icon::Undo,
                            can_undo,
                            undo_tip.as_str(),
                            CornerRadius { nw: 6, sw: 6, ne: 0, se: 0 },
                            false,
                        ),
                        (
                            Icon::Redo,
                            can_redo,
                            redo_tip.as_str(),
                            CornerRadius { nw: 0, sw: 0, ne: 6, se: 6 },
                            true,
                        ),
                    ];
                    for (icon, enabled, tip, corner, is_redo) in segments {
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(30.0, TB_H), egui::Sense::click());
                        let hovered = enabled && resp.hovered();
                        if ui.is_rect_visible(rect) {
                            let bg = if hovered { p.toolbar_btn_hover } else { p.toolbar_btn };
                            let border = if !enabled {
                                disabled_border
                            } else if hovered {
                                p.primary
                            } else {
                                enabled_border
                            };
                            let icon_col = if enabled { Color32::WHITE } else { disabled_icon };
                            ui.painter().rect(
                                rect,
                                corner,
                                bg,
                                Stroke::new(1.0_f32, border),
                                egui::StrokeKind::Inside,
                            );
                            if is_redo {
                                ui.painter().vline(
                                    rect.min.x,
                                    rect.y_range(),
                                    Stroke::new(1.0_f32, split_div),
                                );
                            }
                            paint_icon(
                                ui.painter(),
                                rect.shrink2(egui::vec2(8.0, 6.0)),
                                icon,
                                icon_col,
                            );
                        }
                        if hovered {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if resp.on_hover_text(tip).clicked() && enabled {
                            if is_redo {
                                trigger_redo = true;
                            } else {
                                trigger_undo = true;
                            }
                        }
                    }
                });
                if trigger_undo {
                    app.undo();
                } else if trigger_redo {
                    app.redo();
                }
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
                        app.measure_locked = false;
                    }
                }

                // CAD Planning Tools Dropdown (Gap Closer, Parallel Tracks, NEM Clearance, Reachability Heatmap)
                let is_de_cad = app.lang == Lang::De;
                let cad_lbl = if is_de_cad { "CAD-Tools" } else { "CAD" };
                let cad_w = stable_text_width(ui, "CAD-Tools", "CAD", Some(12.0), false) + 26.0;
                let cad_popup_id = egui::Id::new("tb_cad_tools_popup");
                let cad_open = ui.memory(|m| m.is_popup_open(cad_popup_id));
                let (cad_rect, cad_resp) =
                    ui.allocate_exact_size(egui::vec2(cad_w, TB_H), egui::Sense::click());
                if ui.is_rect_visible(cad_rect) {
                    let bg = if cad_open || cad_resp.hovered() {
                        p.toolbar_btn_hover
                    } else {
                        p.toolbar_btn
                    };
                    let stroke = if cad_open || cad_resp.hovered() {
                        Stroke::new(1.0_f32, p.primary)
                    } else {
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 35))
                    };
                    ui.painter().rect(cad_rect, CornerRadius::same(6), bg, stroke, egui::StrokeKind::Inside);
                    ui.painter().text(
                        egui::pos2(cad_rect.min.x + 8.0, cad_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        cad_lbl,
                        egui::FontId::proportional(12.0),
                        p.toolbar_text,
                    );
                    paint_chevron(
                        ui.painter(),
                        egui::pos2(cad_rect.max.x - 10.0, cad_rect.center().y),
                        ChevronDir::Down,
                        2.8,
                        p.toolbar_text,
                    );
                }
                if cad_resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if cad_resp.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(cad_popup_id));
                }
                egui::popup_below_widget(
                    ui,
                    cad_popup_id,
                    &cad_resp,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui| {
                        style_toolbar_ui(ui, p);
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
                            ui.memory_mut(|m| m.close_popup());
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
                            ui.memory_mut(|m| m.close_popup());
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
                    },
                );

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
                ui.separator();

                let seg_bg = |hovered: bool, active: bool| -> Color32 {
                    if active {
                        if hovered {
                            Color32::from_rgb(3, 120, 182)
                        } else {
                            p.primary_hover
                        }
                    } else if hovered {
                        p.toolbar_btn_hover
                    } else {
                        p.toolbar_btn
                    }
                };
                let split_border = Color32::from_rgba_unmultiplied(148, 163, 184, 38);
                let split_div = Color32::from_rgba_unmultiplied(148, 163, 184, 52);

                // Attached Rotation Split Group ([ ↺ | ↻ 15° | v ]) + Step Lock
                let rot_popup_id = egui::Id::new("tb_rot_step_popup");
                let rot_open = ui.memory(|m| m.is_popup_open(rot_popup_id));
                let mut rot_anchor_resp: Option<egui::Response> = None;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    let (ccw_r, ccw_resp) =
                        ui.allocate_exact_size(egui::vec2(26.0, TB_H), egui::Sense::click());
                    if ui.is_rect_visible(ccw_r) {
                        ui.painter().rect(
                            ccw_r,
                            CornerRadius { nw: 6, sw: 6, ne: 0, se: 0 },
                            seg_bg(ccw_resp.hovered(), false),
                            Stroke::new(1.0_f32, if ccw_resp.hovered() { p.primary } else { split_border }),
                            egui::StrokeKind::Inside,
                        );
                        paint_icon(ui.painter(), ccw_r.shrink2(egui::vec2(6.0, 6.0)), Icon::RotateCcw, Color32::WHITE);
                    }
                    if ccw_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if ccw_resp.on_hover_text(t.rotate_ccw).clicked() {
                        app.rotate_selected(-app.rotation_step);
                    }

                    let (cw_r, cw_resp) =
                        ui.allocate_exact_size(egui::vec2(56.0, TB_H), egui::Sense::click());
                    if ui.is_rect_visible(cw_r) {
                        ui.painter().rect(
                            cw_r,
                            CornerRadius::ZERO,
                            seg_bg(cw_resp.hovered(), false),
                            Stroke::new(1.0_f32, if cw_resp.hovered() { p.primary } else { split_border }),
                            egui::StrokeKind::Inside,
                        );
                        let icon_box = egui::Rect::from_min_size(
                            egui::pos2(cw_r.min.x + 5.0, cw_r.center().y - 6.5),
                            egui::vec2(13.0, 13.0),
                        );
                        paint_icon(ui.painter(), icon_box, Icon::RotateCw, Color32::WHITE);
                        ui.painter().text(
                            egui::pos2(cw_r.max.x - 5.0, cw_r.center().y),
                            egui::Align2::RIGHT_CENTER,
                            format!("{}°", app.rotation_step),
                            egui::FontId::proportional(11.5),
                            Color32::WHITE,
                        );
                        ui.painter().vline(cw_r.min.x, cw_r.y_range(), Stroke::new(1.0_f32, split_div));
                    }
                    if cw_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    let cw_resp = cw_resp.on_hover_text(t.rotate_cw);
                    if cw_resp.clicked() {
                        app.rotate_selected(app.rotation_step);
                    } else if cw_resp.secondary_clicked() {
                        app.rotate_selected(-app.rotation_step);
                    }

                    let (dd_r, dd_resp) =
                        ui.allocate_exact_size(egui::vec2(16.0, TB_H), egui::Sense::click());
                    if ui.is_rect_visible(dd_r) {
                        ui.painter().rect(
                            dd_r,
                            CornerRadius { nw: 0, sw: 0, ne: 6, se: 6 },
                            seg_bg(dd_resp.hovered() || rot_open, false),
                            Stroke::new(
                                1.0_f32,
                                if dd_resp.hovered() || rot_open { p.primary } else { split_border },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().vline(dd_r.min.x, dd_r.y_range(), Stroke::new(1.0_f32, split_div));
                        paint_chevron(ui.painter(), dd_r.center(), ChevronDir::Down, 2.7, Color32::WHITE);
                    }
                    if dd_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    let dd_resp = dd_resp.on_hover_text("Rotation step");
                    if dd_resp.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(rot_popup_id));
                    }
                    rot_anchor_resp = Some(dd_resp);
                });
                if let Some(anchor) = rot_anchor_resp {
                    egui::popup_below_widget(
                        ui,
                        rot_popup_id,
                        &anchor,
                        egui::PopupCloseBehavior::CloseOnClickOutside,
                        |ui| {
                            style_toolbar_ui(ui, p);
                            ui.set_min_width(120.0);
                            for deg in [5.7, 7.5, 15.0, 24.3, 30.0, 45.0, 90.0] {
                                if ui.selectable_value(&mut app.rotation_step, deg, format!("{deg}°")).changed() {
                                    app.save_current_settings();
                                    ui.memory_mut(|m| m.close_popup());
                                }
                            }
                            ui.separator();
                            let lock_menu_lbl = if app.lang == Lang::De {
                                "Winkelschritt einrasten"
                            } else {
                                "Lock step angle"
                            };
                            if ui.checkbox(&mut app.rotation_snap_locked, lock_menu_lbl).changed() {
                                app.save_current_settings();
                            }
                        },
                    );
                }

                let lock_tip = if app.lang == Lang::De {
                    "Rotationsschritt beim Platzieren/Snappen einrasten"
                } else {
                    "Lock rotation step snap angle"
                };
                let snap_lock_icon = if app.rotation_snap_locked { Icon::Lock } else { Icon::Unlock };
                if tb_icon(ui, snap_lock_icon, app.rotation_snap_locked, lock_tip) {
                    app.rotation_snap_locked = !app.rotation_snap_locked;
                    app.save_current_settings();
                }
                ui.separator();

                // Colors Split Group ([ Palette | v ]) + Labels + Power Sim Split Group ([ Zap | v ])
                let colors_active = app.color_mode != ColorMode::Off;
                let colors_popup_id = egui::Id::new("tb_colors_popup");
                let colors_open = ui.memory(|m| m.is_popup_open(colors_popup_id));
                let mut colors_anchor_resp: Option<egui::Response> = None;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let (main_r, main_resp) =
                        ui.allocate_exact_size(egui::vec2(28.0, TB_H), egui::Sense::click());
                    if ui.is_rect_visible(main_r) {
                        ui.painter().rect(
                            main_r,
                            CornerRadius { nw: 6, sw: 6, ne: 0, se: 0 },
                            seg_bg(main_resp.hovered(), colors_active),
                            Stroke::new(
                                1.0_f32,
                                if colors_active || main_resp.hovered() { p.primary } else { split_border },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        paint_icon(ui.painter(), main_r.shrink2(egui::vec2(7.0, 6.0)), Icon::Palette, Color32::WHITE);
                    }
                    if main_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    let main_resp = main_resp.on_hover_text(t.colors_btn);
                    if main_resp.clicked() {
                        app.color_mode = match app.color_mode {
                            ColorMode::Types => ColorMode::Layers,
                            ColorMode::Layers => ColorMode::Off,
                            ColorMode::Off => ColorMode::Types,
                        };
                        app.save_current_settings();
                    }

                    let (dd_r, dd_resp) =
                        ui.allocate_exact_size(egui::vec2(16.0, TB_H), egui::Sense::click());
                    if ui.is_rect_visible(dd_r) {
                        ui.painter().rect(
                            dd_r,
                            CornerRadius { nw: 0, sw: 0, ne: 6, se: 6 },
                            seg_bg(dd_resp.hovered() || colors_open, colors_active),
                            Stroke::new(
                                1.0_f32,
                                if colors_active || dd_resp.hovered() || colors_open {
                                    p.primary
                                } else {
                                    split_border
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().vline(dd_r.min.x, dd_r.y_range(), Stroke::new(1.0_f32, split_div));
                        paint_chevron(ui.painter(), dd_r.center(), ChevronDir::Down, 2.7, Color32::WHITE);
                    }
                    if dd_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    let dd_resp = dd_resp.on_hover_text(t.colors_btn);
                    if dd_resp.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(colors_popup_id));
                    }
                    colors_anchor_resp = Some(dd_resp);
                });
                if let Some(anchor) = colors_anchor_resp {
                    egui::popup_below_widget(
                        ui,
                        colors_popup_id,
                        &anchor,
                        egui::PopupCloseBehavior::CloseOnClickOutside,
                        |ui| {
                            style_toolbar_ui(ui, p);
                            let mut c_changed = false;
                            c_changed |= ui.radio_value(&mut app.color_mode, ColorMode::Types, t.color_types).changed();
                            c_changed |= ui.radio_value(&mut app.color_mode, ColorMode::Layers, t.color_layers).changed();
                            c_changed |= ui.radio_value(&mut app.color_mode, ColorMode::Off, t.color_off).changed();
                            if c_changed {
                                app.save_current_settings();
                                ui.memory_mut(|m| m.close_popup());
                            }
                        },
                    );
                }

                if tb_icon(ui, Icon::Tag, app.show_labels, t.labels) {
                    app.show_labels = !app.show_labels;
                    app.save_current_settings();
                }

                // Power Sim / Voltage Split Group ([ Zap | v ])
                let power_popup_id = egui::Id::new("tb_power_popup");
                let power_open = ui.memory(|m| m.is_popup_open(power_popup_id));
                let mut power_anchor_resp: Option<egui::Response> = None;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let (main_r, main_resp) =
                        ui.allocate_exact_size(egui::vec2(28.0, TB_H), egui::Sense::click());
                    if ui.is_rect_visible(main_r) {
                        ui.painter().rect(
                            main_r,
                            CornerRadius { nw: 6, sw: 6, ne: 0, se: 0 },
                            seg_bg(main_resp.hovered(), app.power_sim_enabled),
                            Stroke::new(
                                1.0_f32,
                                if app.power_sim_enabled || main_resp.hovered() {
                                    p.primary
                                } else {
                                    split_border
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        paint_icon(ui.painter(), main_r.shrink2(egui::vec2(7.0, 6.0)), Icon::Zap, Color32::WHITE);
                    }
                    if main_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    let main_resp = main_resp.on_hover_text(t.volt_btn);
                    if main_resp.clicked() {
                        app.power_sim_enabled = !app.power_sim_enabled;
                    }

                    let (dd_r, dd_resp) =
                        ui.allocate_exact_size(egui::vec2(16.0, TB_H), egui::Sense::click());
                    if ui.is_rect_visible(dd_r) {
                        ui.painter().rect(
                            dd_r,
                            CornerRadius { nw: 0, sw: 0, ne: 6, se: 6 },
                            seg_bg(dd_resp.hovered() || power_open, app.power_sim_enabled),
                            Stroke::new(
                                1.0_f32,
                                if app.power_sim_enabled || dd_resp.hovered() || power_open {
                                    p.primary
                                } else {
                                    split_border
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().vline(dd_r.min.x, dd_r.y_range(), Stroke::new(1.0_f32, split_div));
                        paint_chevron(ui.painter(), dd_r.center(), ChevronDir::Down, 2.7, Color32::WHITE);
                    }
                    if dd_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    let dd_resp = dd_resp.on_hover_text(t.volt_btn);
                    if dd_resp.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(power_popup_id));
                    }
                    power_anchor_resp = Some(dd_resp);
                });
                if let Some(anchor) = power_anchor_resp {
                    let is_de = app.lang == Lang::De;
                    egui::popup_below_widget(
                        ui,
                        power_popup_id,
                        &anchor,
                        egui::PopupCloseBehavior::CloseOnClickOutside,
                        |ui| {
                            style_toolbar_ui(ui, p);
                            for (lbl_de, lbl_en, v, ma) in [
                                ("10V Spur Z (100 mA)", "10V Scale Z (100 mA)", 10.0, 100.0),
                                ("12V Spur N (100 mA)", "12V Scale N (100 mA)", 12.0, 100.0),
                                ("14V Spur TT (300 mA)", "14V Scale TT (300 mA)", 14.0, 300.0),
                                ("14V H0 DCC (450 mA)", "14V H0 DCC (450 mA)", 14.0, 450.0),
                                ("16V H0 Standard (450 mA)", "16V H0 Standard (450 mA)", 16.0, 450.0),
                                ("16V Spur 0 (600 mA)", "16V Scale 0 (600 mA)", 16.0, 600.0),
                                ("18V Spur 1 (800 mA)", "18V Scale 1 (800 mA)", 18.0, 800.0),
                                ("24V Spur G / LGB (1000 mA)", "24V Scale G / LGB (1000 mA)", 24.0, 1000.0),
                            ] {
                                if ui.button(if is_de { lbl_de } else { lbl_en }).clicked() {
                                    app.layout.feed_voltage = Some(v);
                                    app.layout.loco_current_ma = Some(ma);
                                    app.power_sim_enabled = true;
                                    app.mark_layout_dirty();
                                    ui.memory_mut(|m| m.close_popup());
                                }
                            }
                            ui.separator();
                            if ui.button(t.power_dialog).clicked() {
                                app.show_power_dialog = true;
                                ui.memory_mut(|m| m.close_popup());
                            }
                        },
                    );
                }

                    let is_de = app.lang == Lang::De;

                    // Red 2-Wire Short-Circuit alert pill (ALWAYS visible on 2-wire shorts, even when Power Overlay is off!)
                    let first_sc = app
                        .power_sim_cache
                        .as_ref()
                        .filter(|(_, s)| s.has_short_circuit && (s.is_two_wire || app.power_sim_enabled))
                        .map(|(_, s)| {
                            let ids: Vec<String> = s
                                .short_circuit_points
                                .iter()
                                .flat_map(|pt| [pt.piece_id_1.clone(), pt.piece_id_2.clone()])
                                .collect();
                            (
                                s.is_two_wire,
                                s.short_circuit_points.len(),
                                s.short_circuit_points.first().map(|pt| (pt.x, pt.y)),
                                ids,
                            )
                        });
                    if let Some((is_two_wire, cnt, maybe_pt, sc_ids)) = first_sc {
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
                            if !sc_ids.is_empty() {
                                app.selected.clear();
                                for id in sc_ids {
                                    app.selected.insert(id);
                                }
                            }
                            if let Some((x, y)) = maybe_pt {
                                app.focus_world_point(x, y);
                            }
                        }
                    }

                ui.separator();
                // Unified Drive, Cab View & Signal Box Window (for 2nd Monitor)
                let drive_de = "Fahren & Stellwerk";
                let drive_en = "Drive & Signal Box";
                let drive_lbl = if is_de { drive_de } else { drive_en };
                // Button state follows the window; the simulation may keep running after closing it.
                let sim_live = app.vehicle_sim.running && !app.vehicle_sim.vehicles.is_empty();
                let drive_active = app.show_cab_window;
                let drive_w = stable_text_width(ui, drive_de, drive_en, Some(11.0), true) + 32.0;
                let (drive_r, drive_resp) =
                    ui.allocate_exact_size(egui::vec2(drive_w, TB_H), egui::Sense::click());
                if ui.is_rect_visible(drive_r) {
                    let fill = if drive_active && sim_live {
                        Color32::from_rgb(22, 163, 74)
                    } else if drive_active {
                        Color32::from_rgb(2, 132, 199)
                    } else if drive_resp.hovered() {
                        p.toolbar_btn_hover
                    } else {
                        p.toolbar_btn
                    };
                    let stroke = if drive_active && sim_live {
                        Stroke::new(1.0_f32, Color32::from_rgb(74, 222, 128))
                    } else if drive_active || drive_resp.hovered() {
                        Stroke::new(1.0_f32, p.primary)
                    } else {
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 35))
                    };
                    let txt_col = if drive_active { Color32::WHITE } else { p.toolbar_text };
                    ui.painter().rect(drive_r, CornerRadius::same(6), fill, stroke, egui::StrokeKind::Inside);
                    if !drive_active && sim_live {
                        ui.painter().circle_filled(
                            drive_r.right_top() + egui::vec2(-6.0, 6.0),
                            3.0,
                            Color32::from_rgb(74, 222, 128),
                        );
                    }
                    let train_box = egui::Rect::from_min_size(
                        egui::pos2(drive_r.min.x + 7.0, drive_r.center().y - 6.5),
                        egui::vec2(13.0, 13.0),
                    );
                    paint_icon(ui.painter(), train_box, Icon::Train, txt_col);
                    ui.painter().text(
                        egui::pos2(train_box.max.x + 5.0, drive_r.center().y),
                        egui::Align2::LEFT_CENTER,
                        drive_lbl,
                        egui::FontId::proportional(11.0),
                        txt_col,
                    );
                }
                if drive_resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let mut drive_tip = if is_de {
                    "Fahrbetrieb, Fuehrerstand & Gleisbildstellwerk in einem Fenster (fuer 2. Monitor)".to_string()
                } else {
                    "Open Drive, 3D Cab View & Signal Box (Stellwerk) in one window (for a 2nd monitor)".to_string()
                };
                if !drive_active && sim_live {
                    drive_tip.push_str(if is_de { " (Simulation läuft)" } else { " (simulation running)" });
                }
                if drive_resp.on_hover_text(drive_tip).clicked() {
                    app.show_cab_window = !app.show_cab_window;
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
                        app.app_settings.theme_mode = if app.dark_mode {
                            crate::settings::ThemeMode::Dark
                        } else {
                            crate::settings::ThemeMode::Light
                        };
                        app.save_current_settings();
                    }
                    let collab_tip = if app.collab.connected {
                        if app.lang == Lang::De {
                            format!(
                                "Live-Zusammenarbeit · Raum '{}' · {} Teilnehmer",
                                app.collab.room_id,
                                1 + app.collab.users.len()
                            )
                        } else {
                            format!(
                                "Live Collaboration · Room '{}' · {} participant(s)",
                                app.collab.room_id,
                                1 + app.collab.users.len()
                            )
                        }
                    } else {
                        "Live Collaboration".to_string()
                    };
                    if app.collab.connected {
                        let cnt_txt = format!("{}", 1 + app.collab.users.len());
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!("👥 {cnt_txt}"))
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(16, 185, 129))
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .on_hover_text(collab_tip)
                            .clicked()
                        {
                            app.show_collab_dialog = !app.show_collab_dialog;
                        }
                    } else if tb_icon(ui, Icon::Users, false, collab_tip.as_str()) {
                        app.show_collab_dialog = !app.show_collab_dialog;
                    }
                    if pill_btn(ui, if app.lang == Lang::De { "DE" } else { "EN" }, false).clicked() {
                        app.lang = if app.lang == Lang::De { Lang::En } else { Lang::De };
                        app.save_current_settings();
                    }
                });
            });
        });
    let bottom_line_col = if app.dark_mode {
        p.border_main
    } else {
        Color32::from_rgba_unmultiplied(15, 23, 42, 95)
    };
    let r = qt_resp.response.rect;
    ctx.layer_painter(qt_resp.response.layer_id).hline(
        r.x_range(),
        r.bottom(),
        Stroke::new(1.0_f32, bottom_line_col),
    );
}
