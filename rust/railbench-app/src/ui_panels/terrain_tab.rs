use super::*;

pub(super) fn draw_terrain_tab(ui: &mut egui::Ui, app: &mut RailbenchApp) {
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

