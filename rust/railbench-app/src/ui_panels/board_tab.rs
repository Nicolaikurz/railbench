use super::*;

pub(super) fn draw_board_tab(ui: &mut egui::Ui, app: &mut RailbenchApp) {
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
                let mut all_enabled = boards_snapshot.iter().any(|b| b.enabled);
                if ui
                    .checkbox(
                        &mut all_enabled,
                        if is_de { "Grundplatten aktiviert (Alle)" } else { "Enable Baseboards (All)" },
                    )
                    .changed()
                {
                    app.push_undo();
                    if let Some(bs) = app.layout.baseboards.as_mut() {
                        for b in bs.iter_mut() {
                            b.enabled = all_enabled;
                        }
                    }
                    changed = true;
                }
                ui.add_space(3.0);
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
                            let name_buf = board.name.get_or_insert_with(|| {
                                if is_de { "Tisch" } else { "Table" }.into()
                            });
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
                            for (lbl, shape, w, d) in [
                                ("Compact 180x90", "rect", 1800.0, 900.0),
                                ("Std 200x100", "rect", 2000.0, 1000.0),
                                ("240x120", "rect", 2400.0, 1200.0),
                                ("Studio 300x150", "rect", 3000.0, 1500.0),
                                ("L-Shape", "l-shape", 2600.0, 1800.0),
                                ("U-Shape", "u-shape", 3200.0, 2000.0),
                                ("T-Shape", "t-shape", 2800.0, 1800.0),
                                ("Corner Tri", "triangle", 1800.0, 1800.0),
                            ] {
                                if ui.small_button(lbl).clicked() {
                                    board.shape = Some(shape.into());
                                    board.width = w;
                                    board.depth = d;
                                    board.enabled = true;
                                    board_tools::apply_shape_defaults(board);
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
                                if ui.small_button(if is_de { "+ Platte" } else { "+ Plate" }).clicked() {
                                    if let Some(ref tb) = ref_board {
                                        let cnt = layer.plates.as_ref().map(|v| v.len()).unwrap_or(0) + 1;
                                        let plate_name = if is_de {
                                            format!("Platte {cnt}")
                                        } else {
                                            format!("Plate {cnt}")
                                        };
                                        let np = board_tools::make_layer_plate(tb, layer, plate_name, &mut app.next_piece_seq);
                                        layer.plates.get_or_insert_with(Vec::new).push(np);
                                        board_tools::sync_wood_mode(layer);
                                        changed = true;
                                    }
                                }
                                if ui.small_button(if is_de { "Platte zeichnen" } else { "Sketch Plate" }).clicked() {
                                    plate_sketch_req = Some((lid.clone(), cur_id.clone(), None, SketchMode::Poly, true));
                                }
                                if ui.small_button(if is_de { "Rechteck-Platte" } else { "Rect Plate" }).clicked() {
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
                                        if ui.small_button(if is_de { "Wie Tisch" } else { "Fit Table" }).clicked() {
                                            if let Some(ref tb) = ref_board {
                                                board_tools::copy_board_shape_to_plate(tb, plate);
                                                changed = true;
                                            }
                                        }
                                        if ui.small_button(if is_de { "Bearb." } else { "Edit" }).clicked() {
                                            plate_sketch_req = Some((lid.clone(), cur_id.clone(), Some(pid.clone()), SketchMode::Edit, false));
                                        }
                                        if ui.small_button(if is_de { "Entf." } else { "Del" }).clicked() {
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
                                        let pt = plate.thickness_mm.get_or_insert(12.0);
                                        ui.label("T:");
                                        changed |= ui.add(egui::DragValue::new(pt).range(3.0..=40.0).speed(0.5)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        let pstyle = plate.style.get_or_insert_with(|| "plywood".into());
                                        ui.label("Style:");
                                        egui::ComboBox::from_id_salt(format!("plate_style_{pid}"))
                                            .width(95.0)
                                            .selected_text(pstyle.as_str())
                                            .show_ui(ui, |ui| {
                                                for st in ["plywood", "wood", "dark-wood", "slate", "white", "grass"] {
                                                    if ui.selectable_label(pstyle.as_str() == st, st).clicked() {
                                                        *pstyle = st.into();
                                                        changed = true;
                                                    }
                                                }
                                            });
                                        if ui.small_button("Redraw Poly").clicked() {
                                            plate_sketch_req = Some((lid.clone(), cur_id.clone(), Some(pid.clone()), SketchMode::Poly, false));
                                        }
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

