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
        // 1. Unified Continuous Heightfield Brush Card (works in both 3D & 2D!)
        ui.label(
            RichText::new(if is_de { "GELAENDE-EDITOR (3D & 2D)" } else { "TERRAIN SCULPTOR (3D & 2D)" })
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
                let (_active_lid, active_y0, active_editable, active_name) =
                    terrain_tools::active_layer_info(&app.layout);

                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(if is_de {
                            format!("Aktive Ebene: {active_name} ({active_y0:.0} mm)")
                        } else {
                            format!("Active Layer: {active_name} ({active_y0:.0} mm)")
                        })
                        .size(10.5)
                        .strong()
                        .color(if active_editable {
                            Color32::from_rgb(56, 189, 248)
                        } else {
                            Color32::from_rgb(245, 158, 11)
                        }),
                    );
                    if !active_editable {
                        ui.label(
                            RichText::new(if is_de {
                                "🔒 Aktive Ebene ist gesperrt/ausgeblendet"
                            } else {
                                "🔒 Active layer is locked/hidden"
                            })
                            .size(10.0)
                            .color(Color32::from_rgb(245, 158, 11)),
                        );
                    }
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    let sculpt_3d_active = app.is_3d_mode && cur_active && app.terrain_brush.is_heightfield_mode();
                    if mode_btn(
                        ui,
                        sculpt_3d_active,
                        if is_de { "3D Formen" } else { "Sculpt in 3D" },
                        if is_de {
                            "Direkt in der 3D-Ansicht modellieren (Shift: Glaetten, Alt: Invertieren)"
                        } else {
                            "Switch to 3D viewport and sculpt heightfield live"
                        },
                        Color32::from_rgb(2, 132, 199),
                    ) {
                        app.is_3d_mode = true;
                        app.measure_mode = false;
                        app.sketcher.stop();
                        if !app.terrain_brush.active || !app.terrain_brush.is_heightfield_mode() {
                            app.terrain_brush.set_active(true, Some(TerrainBrushMode::SculptRaise), None);
                        }
                    }
                    if cur_active
                        && ui
                            .small_button(if is_de { "Beenden" } else { "Stop" })
                            .clicked()
                    {
                        app.terrain_brush.set_active(false, None, None);
                    }
                });
                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    if mode_btn(
                        ui,
                        cur_active && cur_mode == TerrainBrushMode::SculptRaise,
                        if is_de { "Berg +" } else { "Raise" },
                        if is_de {
                            "Gelaende organisch anheben (Alt: Absenken, Shift: Glaetten)"
                        } else {
                            "Raise terrain organically (Alt: Lower, Shift: Smooth)"
                        },
                        Color32::from_rgb(22, 163, 74),
                    ) {
                        activate_mode(app, TerrainBrushMode::SculptRaise);
                    }
                    if mode_btn(
                        ui,
                        cur_active && cur_mode == TerrainBrushMode::SculptLower,
                        if is_de { "Tal -" } else { "Lower" },
                        if is_de {
                            "Taeler und Flussbetten absenken"
                        } else {
                            "Carve valleys and basins downward"
                        },
                        Color32::from_rgb(234, 88, 12),
                    ) {
                        activate_mode(app, TerrainBrushMode::SculptLower);
                    }
                    if mode_btn(
                        ui,
                        cur_active && cur_mode == TerrainBrushMode::SculptSmooth,
                        if is_de { "Glaetten" } else { "Smooth" },
                        if is_de {
                            "Haenge weich ueberblenden"
                        } else {
                            "Soften slopes and ridges"
                        },
                        Color32::from_rgb(14, 116, 144),
                    ) {
                        activate_mode(app, TerrainBrushMode::SculptSmooth);
                    }
                    if mode_btn(
                        ui,
                        cur_active && cur_mode == TerrainBrushMode::SculptFlatten,
                        "Plateau",
                        if is_de {
                            "Auf geklickte Ausgangshoehe einebnen"
                        } else {
                            "Flatten terrain to clicked sample height"
                        },
                        Color32::from_rgb(202, 138, 4),
                    ) {
                        activate_mode(app, TerrainBrushMode::SculptFlatten);
                    }
                });
                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    if mode_btn(
                        ui,
                        cur_active && cur_mode == TerrainBrushMode::SculptGrade,
                        if is_de { "Bahndamm" } else { "Grade Track" },
                        if is_de {
                            "Gelaende sanft bis unter naechste Gleis-Rampe anheben"
                        } else {
                            "Build sloped embankment cleanly up to nearby tracks"
                        },
                        Color32::from_rgb(126, 34, 206),
                    ) {
                        activate_mode(app, TerrainBrushMode::SculptGrade);
                    }
                    if mode_btn(
                        ui,
                        cur_active && cur_mode == TerrainBrushMode::SculptBiome,
                        if is_de { "Biom" } else { "Biome" },
                        if is_de {
                            "Gras, Wald, Fels, Schnee, Sand, Erde oder Wasser malen"
                        } else {
                            "Paint grass, forest, rock, snow, sand, dirt, or water"
                        },
                        Color32::from_rgb(13, 148, 136),
                    ) {
                        activate_mode(app, TerrainBrushMode::SculptBiome);
                    }
                    if mode_btn(
                        ui,
                        cur_active && matches!(cur_mode, TerrainBrushMode::SculptReset | TerrainBrushMode::Erase),
                        if is_de { "Radieren" } else { "Reset" },
                        if is_de {
                            "Gelaende lokal auf Holzplatte zuruecksetzen"
                        } else {
                            "Reset terrain back to bare baseboard height"
                        },
                        Color32::from_rgb(185, 28, 28),
                    ) {
                        activate_mode(app, TerrainBrushMode::SculptReset);
                    }
                });

                if app.terrain_brush.active {
                    ui.add_space(5.0);
                    let hint = match app.terrain_brush.mode {
                        TerrainBrushMode::SculptFlatten => {
                            if is_de {
                                "Startpunkt bestimmt Zielhoehe. Shift: Glaetten · Rechtsklick: Beenden/Orbit."
                            } else {
                                "Click point sets plateau elevation. Shift: Smooth · Right-drag in 3D: Orbit."
                            }
                        }
                        TerrainBrushMode::SculptGrade => {
                            if is_de {
                                "Ueber erhoehte Gleise ziehen fuer natuerlichen Bahndamm."
                            } else {
                                "Stroke along elevated tracks to raise an embankment."
                            }
                        }
                        TerrainBrushMode::SculptBiome => {
                            if is_de {
                                "Biom waehlen und ueber das Gelaende malen (Wasser grabe Becken)."
                            } else {
                                "Pick biome below and paint onto terrain (Water carves a basin)."
                            }
                        }
                        _ => {
                            if is_de {
                                "Ziehen zum Formen · Shift: Glaetten · Alt: Invertieren · Shift/Ctrl+Rad: R/Staerke."
                            } else {
                                "Drag to sculpt · Shift: Smooth · Alt: Invert · Shift/Ctrl+Wheel: R/Strength."
                            }
                        }
                    };
                    ui.label(RichText::new(hint).size(10.5).color(Color32::from_rgb(74, 222, 128)));
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Radius:");
                    ui.add(
                        egui::DragValue::new(&mut app.terrain_brush.radius)
                            .range(40.0..=600.0)
                            .speed(5.0)
                            .suffix(" mm"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Staerke:" } else { "Strength:" });
                    let mut pct = (app.terrain_brush.strength * 100.0).round();
                    if ui
                        .add(
                            egui::DragValue::new(&mut pct)
                                .range(10.0..=100.0)
                                .speed(2.0)
                                .suffix("%"),
                        )
                        .changed()
                    {
                        app.terrain_brush.strength = (pct / 100.0).clamp(0.10, 1.0);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Max Hoehe:" } else { "Max Height:" });
                    ui.add(
                        egui::DragValue::new(&mut app.terrain_brush.height)
                            .range(40.0..=560.0)
                            .speed(5.0)
                            .suffix(" mm"),
                    );
                });
                let biome_lbl = |b: u8| match (b, is_de) {
                    (1, true) => "Gras / Wiese",
                    (1, false) => "Grass / Meadow",
                    (2, true) => "Wald / Tannen",
                    (2, false) => "Forest",
                    (3, true) => "Fels / Granit",
                    (3, false) => "Rock / Cliff",
                    (4, true) => "Schnee / Gipfel",
                    (4, false) => "Snow",
                    (5, _) => "Sand",
                    (6, true) => "Erde",
                    (6, false) => "Dirt",
                    (7, true) => "Wasser",
                    (7, false) => "Water",
                    (_, true) => "Auto (Hoehe & Neigung)",
                    (_, false) => "Auto (Elevation & Slope)",
                };
                ui.horizontal(|ui| {
                    ui.label(if is_de { "Biom:" } else { "Biome:" });
                    egui::ComboBox::from_id_salt("brush_hf_biome")
                        .width(155.0)
                        .selected_text(biome_lbl(app.terrain_brush.biome))
                        .show_ui(ui, |ui| {
                            for b in 0..=7_u8 {
                                ui.selectable_value(&mut app.terrain_brush.biome, b, biome_lbl(b));
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
            });

        ui.add_space(10.0);

        // 2. Quick Unified Heightfield Stamping & Global Actions
        ui.label(
            RichText::new(if is_de { "SCHNELL-AKTIONEN (HOEHENFELD)" } else { "QUICK TERRAIN ACTIONS" })
                .strong()
                .size(11.0)
                .color(p.text_muted),
        );
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let max_h = app.terrain_brush.height as f32;
            if ui.button(if is_de { "+ Berg" } else { "+ Mountain" }).clicked() {
                app.push_undo();
                if terrain_tools::stamp_preset_into_heightfield(&mut app.layout, "single", max_h) {
                    if app.terrain_brush.auto_tunnel {
                        terrain_tools::apply_auto_tunnel_for_heightfield(&mut app.layout);
                    }
                    app.mark_layout_dirty();
                }
            }
            if ui.button(if is_de { "+ Eck-Massiv" } else { "+ Corner Massif" }).clicked() {
                app.push_undo();
                if terrain_tools::stamp_preset_into_heightfield(&mut app.layout, "corner", max_h) {
                    if app.terrain_brush.auto_tunnel {
                        terrain_tools::apply_auto_tunnel_for_heightfield(&mut app.layout);
                    }
                    app.mark_layout_dirty();
                }
            }
            if ui.button(if is_de { "+ Kulissen-Kette" } else { "+ Backdrop Ridge" }).clicked() {
                app.push_undo();
                if terrain_tools::stamp_preset_into_heightfield(&mut app.layout, "ridge", max_h) {
                    if app.terrain_brush.auto_tunnel {
                        terrain_tools::apply_auto_tunnel_for_heightfield(&mut app.layout);
                    }
                    app.mark_layout_dirty();
                }
            }
            if ui.button(if is_de { "Platte begruenen" } else { "Fill Board Grass" }).clicked() {
                app.push_undo();
                if terrain_tools::fill_heightfield_biome(&mut app.layout, railbench_core::terrain::BIOME_GRASS) {
                    app.mark_layout_dirty();
                }
            }
            if terrain_tools::active_layer_heightfield(&app.layout).is_some()
                && ui.button(if is_de { "Alles glaetten" } else { "Smooth All" }).clicked()
            {
                app.push_undo();
                if terrain_tools::smooth_entire_heightfield(&mut app.layout) {
                    app.mark_layout_dirty();
                }
            }
            if terrain_tools::active_layer_heightfield(&app.layout).is_some()
                && ui.button(if is_de { "Hoehenfeld leeren" } else { "Clear Field" }).clicked()
            {
                app.push_undo();
                if terrain_tools::clear_heightfield(&mut app.layout) {
                    app.mark_layout_dirty();
                }
            }
        });

        ui.add_space(8.0);
        ui.label(
            RichText::new(if is_de { "UNTERGRUND / WASSER / LANDSCHAFT" } else { "GROUND TERRAIN / WATER" })
                .strong()
                .size(11.0)
                .color(p.text_muted),
        );
        let _ = (style_label, ground_label);
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

