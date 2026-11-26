use super::*;

/// Processes direct 3D terrain sculpting when `terrain_brush.active && terrain_brush.is_heightfield_mode()`.
/// Returns `(consumed_primary_drag, consumed_scroll, Edit3DOutput)`.
pub fn handle_3d_terrain_sculpt_interaction(
    ui: &mut egui::Ui,
    rect: Rect,
    response: &Response,
    camera: &Camera3D,
    brush: &mut crate::terrain_tools::TerrainBrushState,
    layout: &mut Layout,
) -> (bool, bool, Edit3DOutput) {
    let mut out = Edit3DOutput::default();
    if !brush.active || !brush.is_heightfield_mode() {
        return (false, false, out);
    }

    let (active_lid, active_y0, editable, _) = crate::terrain_tools::active_layer_info(layout);
    let bounds = crate::terrain_tools::active_layer_bounds(layout);
    let hover_pos = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
    let shift_held = ui.input(|i| i.modifiers.shift);
    let alt_held = ui.input(|i| i.modifiers.alt);
    let ctrl_held = ui.input(|i| i.modifiers.ctrl || i.modifiers.mac_cmd);

    let mut consumed_scroll = false;
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.01 {
            if shift_held {
                brush.adjust_radius(if scroll > 0.0 { 15.0 } else { -15.0 });
                consumed_scroll = true;
            } else if ctrl_held {
                brush.adjust_strength(if scroll > 0.0 { 0.08 } else { -0.08 });
                consumed_scroll = true;
            }
        }
    }

    if !editable {
        brush.cursor_world = None;
        brush.is_drawing = false;
        return (false, consumed_scroll, out);
    }

    let boards: Vec<railbench_core::types::BaseboardConfig> =
        crate::board_tools::get_baseboards_slice(layout)
            .into_iter()
            .filter(|b| b.enabled)
            .collect();
    let support_polys = crate::terrain_tools::collect_terrain_support_polygons(
        layout,
        active_lid.as_deref(),
        &boards,
    );
    let (poly_offsets, poly_coords) = crate::terrain_tools::pack_support_polygons(&support_polys);
    let active_hf = crate::terrain_tools::active_layer_heightfield(layout);

    let hit_pt = hover_pos.and_then(|hp| {
        let (ro, rd) = camera.screen_to_ray(rect, hp);
        let ro_local = [ro[0], ro[1] - active_y0 as f32, ro[2]];
        let hit = railbench_core::terrain::raycast_heightfield_3d(
            ro_local,
            rd,
            active_hf,
            bounds,
        )?;
        if poly_offsets.len() >= 2
            && !railbench_core::geometry::is_point_in_any_polygon(
                hit[0] as f64,
                hit[2] as f64,
                &poly_offsets,
                &poly_coords,
            )
        {
            return None;
        }
        Some(hit)
    });
    brush.cursor_world = hit_pt.map(|p| Vec2 {
        x: p[0] as f64,
        y: p[2] as f64,
    });
    if hover_pos.is_some() && hit_pt.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }

    if response.clicked_by(egui::PointerButton::Secondary) {
        brush.set_active(false, None, None);
        return (false, consumed_scroll, out);
    }

    let mut consumed_drag = false;
    if response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(wp) = brush.cursor_world {
            brush.is_drawing = true;
            brush.flatten_target_mm = None;
            brush.last_sculpt_world = None;
            out.push_undo = true;
            if brush.apply_live_sculpt_step(wp, layout, shift_held, alt_held) {
                out.layout_changed = true;
            }
            consumed_drag = true;
        }
    }

    if brush.is_drawing && response.dragged_by(egui::PointerButton::Primary) {
        consumed_drag = true;
        if let Some(wp) = brush.cursor_world {
            if brush.apply_live_sculpt_step(wp, layout, shift_held, alt_held) {
                out.layout_changed = true;
            }
        }
    } else if brush.is_drawing && !response.dragged_by(egui::PointerButton::Primary) {
        brush.is_drawing = false;
        brush.flatten_target_mm = None;
        brush.last_sculpt_world = None;
        if brush.auto_tunnel && crate::terrain_tools::apply_auto_tunnel_for_heightfield(layout) > 0 {
            out.layout_changed = true;
        }
        out.layout_changed = true;
    } else if response.clicked_by(egui::PointerButton::Primary) {
        if let Some(wp) = brush.cursor_world {
            brush.flatten_target_mm = None;
            brush.last_sculpt_world = None;
            out.push_undo = true;
            if brush.apply_live_sculpt_step(wp, layout, shift_held, alt_held) {
                out.layout_changed = true;
            }
            brush.flatten_target_mm = None;
            brush.last_sculpt_world = None;
            if brush.auto_tunnel && crate::terrain_tools::apply_auto_tunnel_for_heightfield(layout) > 0 {
                out.layout_changed = true;
            }
        }
    }

    (consumed_drag, consumed_scroll, out)
}

/// Projects the terrain-conforming 3D brush reticle and draws the floating bottom-center
/// 3D Terrain Sculpt toolbar when unified heightfield sculpting is active.
#[allow(clippy::too_many_arguments)]
pub fn draw_3d_terrain_brush_overlay(
    ui: &mut egui::Ui,
    rect: Rect,
    camera: &Camera3D,
    brush: &mut crate::terrain_tools::TerrainBrushState,
    layout: &Layout,
    dark_mode: bool,
    lang: Lang,
) {
    if !brush.active || !brush.is_heightfield_mode() {
        return;
    }
    let is_de = lang == Lang::De;
    let painter = ui.painter_at(rect);
    let shift_held = ui.input(|i| i.modifiers.shift);
    let alt_held = ui.input(|i| i.modifiers.alt);
    let eff_op = brush.active_sculpt_op(shift_held, alt_held);
    let ring_col = match eff_op {
        crate::terrain_tools::SculptOp::Raise => Color32::from_rgb(34, 197, 94),
        crate::terrain_tools::SculptOp::Lower => Color32::from_rgb(249, 115, 22),
        crate::terrain_tools::SculptOp::Smooth => Color32::from_rgb(56, 189, 248),
        crate::terrain_tools::SculptOp::Flatten(_) => Color32::from_rgb(234, 179, 8),
        crate::terrain_tools::SculptOp::GradeTrack => Color32::from_rgb(168, 85, 247),
        crate::terrain_tools::SculptOp::PaintBiome(_) => Color32::from_rgb(20, 184, 166),
        crate::terrain_tools::SculptOp::Reset => Color32::from_rgb(239, 68, 68),
    };

    let (_active_lid, active_y0, editable, active_layer_name) =
        crate::terrain_tools::active_layer_info(layout);
    if let Some(cw) = brush.cursor_world {
        let active_hf = crate::terrain_tools::active_layer_heightfield(layout);
        let y_base = active_y0 as f32;
        let sample_h = |wx: f64, wz: f64| -> f32 {
            active_hf
                .map(|hf| railbench_core::terrain::sample_heightfield_bilinear(hf, wx, wz))
                .unwrap_or(0.0)
        };
        const N: usize = 28;
        let mut outer = Vec::with_capacity(N + 1);
        let mut inner = Vec::with_capacity(N + 1);
        for i in 0..=N {
            let ang = (i as f64 / N as f64) * std::f64::consts::TAU;
            let ox = cw.x + ang.cos() * brush.radius;
            let oz = cw.y + ang.sin() * brush.radius;
            let oy = y_base + sample_h(ox, oz) + 2.0;
            if let Some(sp) = camera.world_to_screen(rect, [ox as f32, oy, oz as f32]) {
                outer.push(sp);
            }
            let ix = cw.x + ang.cos() * brush.radius * 0.35;
            let iz = cw.y + ang.sin() * brush.radius * 0.35;
            let iy = y_base + sample_h(ix, iz) + 2.5;
            if let Some(sp) = camera.world_to_screen(rect, [ix as f32, iy, iz as f32]) {
                inner.push(sp);
            }
        }
        if outer.len() >= 3 {
            painter.add(egui::Shape::line(outer, Stroke::new(2.4_f32, ring_col)));
        }
        if inner.len() >= 3 {
            painter.add(egui::Shape::line(
                inner,
                Stroke::new(
                    1.4_f32,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 175),
                ),
            ));
        }
        let ch = sample_h(cw.x, cw.y);
        if let Some(csp) = camera.world_to_screen(rect, [cw.x as f32, y_base + ch + 2.5, cw.y as f32]) {
            painter.circle_filled(csp, 4.0, ring_col);
            painter.circle_stroke(csp, 4.8, Stroke::new(1.2_f32, Color32::WHITE));
            let txt = format!("H: {:.0} mm · R: {:.0} mm", ch, brush.radius);
            let pill = Rect::from_min_size(csp + egui::vec2(14.0, -26.0), egui::vec2(146.0, 21.0));
            painter.rect_filled(pill, 6.0, Color32::from_rgba_unmultiplied(15, 23, 42, 225));
            painter.rect_stroke(pill, 6.0, Stroke::new(1.1_f32, ring_col), egui::StrokeKind::Inside);
            painter.text(
                pill.center(),
                egui::Align2::CENTER_CENTER,
                txt,
                FontId::proportional(11.0),
                Color32::WHITE,
            );
        }
    }

    // Floating Bottom-Center 3D Terrain Sculpt HUD Bar
    let border = Color32::from_rgba_unmultiplied(34, 197, 94, 145);
    let h_btn = super::super::viewport::HUD_3D_BTN_H;
    egui::Area::new(egui::Id::new("hud_3d_terrain_sculpt_bar"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -14.0))
        .constrain_to(rect.shrink(6.0))
        .show(ui.ctx(), |ui| {
            super::super::viewport::hud_3d_panel_frame(dark_mode, Some(border)).show(ui, |ui| {
                ui.spacing_mut().interact_size.y = h_btn;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let hud_title = if editable {
                        format!("{} · {active_layer_name}", if is_de { "3D GELAENDE" } else { "3D TERRAIN" })
                    } else {
                        format!("🔒 {active_layer_name}")
                    };
                    Frame::new()
                        .fill(if editable {
                            Color32::from_rgb(22, 163, 74)
                        } else {
                            Color32::from_rgb(217, 119, 6)
                        })
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(hud_title)
                                    .strong()
                                    .size(10.5)
                                    .color(Color32::WHITE),
                            );
                        });
                    ui.separator();
                    let tools = [
                        (crate::terrain_tools::TerrainBrushMode::SculptRaise, if is_de { "Berg +" } else { "Raise" }),
                        (crate::terrain_tools::TerrainBrushMode::SculptLower, if is_de { "Tal -" } else { "Lower" }),
                        (crate::terrain_tools::TerrainBrushMode::SculptSmooth, if is_de { "Glaetten" } else { "Smooth" }),
                        (crate::terrain_tools::TerrainBrushMode::SculptFlatten, "Plateau"),
                        (crate::terrain_tools::TerrainBrushMode::SculptGrade, if is_de { "Damm" } else { "Grade" }),
                        (crate::terrain_tools::TerrainBrushMode::SculptBiome, if is_de { "Biom" } else { "Biome" }),
                        (crate::terrain_tools::TerrainBrushMode::SculptReset, "Reset"),
                    ];
                    for (m, lbl) in tools {
                        if super::super::viewport::hud_3d_pill_button(
                            ui,
                            dark_mode,
                            lbl,
                            brush.mode == m,
                            true,
                            46.0,
                        )
                        .clicked()
                        {
                            brush.mode = m;
                        }
                    }
                    ui.separator();
                    ui.label("R:");
                    ui.add(
                        egui::DragValue::new(&mut brush.radius)
                            .range(40.0..=600.0)
                            .speed(4.0)
                            .suffix(" mm"),
                    );
                    let mut pct = (brush.strength * 100.0).round();
                    if ui
                        .add(
                            egui::DragValue::new(&mut pct)
                                .range(10.0..=100.0)
                                .speed(2.0)
                                .suffix("%"),
                        )
                        .changed()
                    {
                        brush.strength = (pct / 100.0).clamp(0.10, 1.0);
                    }
                    if matches!(
                        brush.mode,
                        crate::terrain_tools::TerrainBrushMode::SculptBiome
                            | crate::terrain_tools::TerrainBrushMode::SculptRaise
                    ) {
                        let biome_lbl = |b: u8| match (b, is_de) {
                            (1, true) => "Gras",
                            (1, false) => "Grass",
                            (2, true) => "Wald",
                            (2, false) => "Forest",
                            (3, true) => "Fels",
                            (3, false) => "Rock",
                            (4, true) => "Schnee",
                            (4, false) => "Snow",
                            (5, _) => "Sand",
                            (6, true) => "Erde",
                            (6, false) => "Dirt",
                            (7, true) => "Wasser",
                            (7, false) => "Water",
                            _ => "Auto",
                        };
                        egui::ComboBox::from_id_salt("hud3d_sculpt_biome")
                            .width(74.0)
                            .selected_text(biome_lbl(brush.biome))
                            .show_ui(ui, |ui| {
                                for b in 0..=7_u8 {
                                    ui.selectable_value(&mut brush.biome, b, biome_lbl(b));
                                }
                            });
                    }
                    ui.separator();
                    if super::super::viewport::hud_3d_stable_toggle(
                        ui,
                        dark_mode,
                        false,
                        if is_de { "Fertig" } else { "Done" },
                        "Fertig",
                        "Done",
                    )
                    .clicked()
                    {
                        brush.set_active(false, None, None);
                    }
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, create_piece, find_entry};
    use eframe::egui::Pos2;
    use railbench_core::types::Transform;

    #[test]
    fn screen_to_ray_and_world_to_screen_roundtrip_accurately() {
        let cam = Camera3D {
            target: [120.0, 20.0, -50.0],
            yaw: 0.7,
            pitch: 0.45,
            distance: 900.0,
        };
        let rect = Rect::from_min_size(Pos2::new(100.0, 50.0), egui::vec2(1200.0, 800.0));
        let world_pt = [260.0_f32, 35.0, 80.0];
        let scr = cam.world_to_screen(rect, world_pt).expect("point in front of camera");

        let (ro, rd) = cam.screen_to_ray(rect, scr);
        let hit = ray_intersect_y_plane(ro, rd, 35.0).expect("ray hits horizontal Y=35 plane");
        assert!(
            (hit[0] - world_pt[0]).abs() < 0.8 && (hit[2] - world_pt[2]).abs() < 0.8,
            "roundtrip mismatch: expected {world_pt:?}, got {hit:?}"
        );
    }

    #[test]
    fn ray_plane_intersection_and_3d_picking_selects_track_and_model() {
        let catalogs = builtin_catalogs();
        let e24188 = find_entry(&catalogs, "24188").unwrap();
        let p0 = create_piece(
            "track-pick".into(),
            e24188,
            Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
            },
            None,
        );
        let layout = Layout {
            name: "PickTest".into(),
            scale: "H0".into(),
            pieces: vec![p0],
            models: Some(vec![railbench_core::types::Model3D {
                id: "m-station".into(),
                name: "Bahnhof".into(),
                category: "Buildings".into(),
                transform: Transform {
                    x: 94.0,
                    y: -140.0,
                    rotation: 0.0,
                },
                elevation: Some(0.0),
                scale: Some(1.0),
                layer_id: None,
                width: 140.0,
                depth: 80.0,
                height: 65.0,
                model_type: "builtin".into(),
                builtin_id: Some("station".into()),
                mountain_style: None,
                mountain_path: None,
                ground_terrain_type: None,
                file_data: None,
                file_name: None,
            }]),
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
            terrain_field: None,
        };

        let cam = Camera3D {
            target: [94.0, 10.0, 0.0],
            yaw: 0.4,
            pitch: 0.65,
            distance: 500.0,
        };
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 700.0));
        let scr = cam.world_to_screen(rect, [94.0, 5.5, 0.0]).unwrap();
        let (ro, rd) = cam.screen_to_ray(rect, scr);
        let picked = pick_3d_object(ro, rd, &layout, &catalogs, true);
        assert_eq!(picked.as_deref(), Some("track-pick"));

        let scr_m = cam.world_to_screen(rect, [94.0, 25.0, -140.0]).unwrap();
        let (mro1, mrd1) = cam.screen_to_ray(rect, scr_m);
        assert_eq!(
            pick_3d_object(mro1, mrd1, &layout, &catalogs, true).as_deref(),
            Some("m-station")
        );

        let miss_scr = cam.world_to_screen(rect, [94.0, 5.5, 250.0]).unwrap();
        let (mro, mrd) = cam.screen_to_ray(rect, miss_scr);
        assert!(pick_3d_object(mro, mrd, &layout, &catalogs, true).is_none());
    }

    #[test]
    fn catalog_drop_preview_3d_snaps_to_open_connector() {
        let catalogs = builtin_catalogs();
        let e24188 = find_entry(&catalogs, "24188").unwrap();
        let p0 = create_piece(
            "anchor-track".into(),
            e24188,
            Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
            },
            None,
        );
        let right_end = &p0.connectors[1];
        let right_x = right_end.x;
        let right_y = right_end.y;
        let layout = Layout {
            name: "Snap3DTest".into(),
            scale: "H0".into(),
            pieces: vec![p0],
            models: None,
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
            terrain_field: None,
        };

        let drop_near_right = Vec2 {
            x: right_x + 90.0,
            y: right_y + 18.0,
        };
        let (ghost, snap_pt, elev_mm) = compute_3d_catalog_drop_preview(
            e24188,
            drop_near_right,
            0.0,
            &layout,
            &catalogs,
            true,
            140.0,
        );
        let snap_xyz = snap_pt.expect("3D drop near open connector must magnetically snap");
        assert!((snap_xyz[0] - right_x as f32).abs() < 1e-2);
        assert!((snap_xyz[2] - right_y as f32).abs() < 1e-2);
        assert!((elev_mm - 0.0).abs() < 1e-3);
        assert!(
            ghost
                .connectors
                .iter()
                .any(|c| (c.x - right_x).hypot(c.y - right_y) < 1e-3),
            "snapped ghost connector must align exactly with target connector"
        );
    }

    #[test]
    fn rotation_handle_3d_projects_valid_point_for_selected_track() {
        let catalogs = builtin_catalogs();
        let e24188 = find_entry(&catalogs, "24188").unwrap();
        let p0 = create_piece(
            "t-rot3d".into(),
            e24188,
            Transform {
                x: 40.0,
                y: -20.0,
                rotation: 0.0,
            },
            None,
        );
        let layout = Layout {
            name: "Rot3DTest".into(),
            scale: "H0".into(),
            pieces: vec![p0],
            models: None,
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
            terrain_field: None,
        };
        let mut selected = HashSet::new();
        selected.insert("t-rot3d".to_string());
        let cam = Camera3D {
            target: [134.0, 10.0, -20.0],
            yaw: 0.35,
            pitch: 0.7,
            distance: 700.0,
        };
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 700.0));
        let edit = Edit3DState::default();
        let (piv, hdl, base_ang, init_rot) =
            compute_3d_rotation_handle_world(&layout, &selected, true, cam.distance, &edit)
                .expect("selected track produces a 3D rotation handle point");
        assert!(piv.iter().all(|v| v.is_finite()) && hdl.iter().all(|v| v.is_finite()));
        assert!((piv[0] - hdl[0]).hypot(piv[2] - hdl[2]) > 40.0);
        assert!((base_ang - 270.0).abs() < 1e-4);
        assert_eq!(init_rot, Some(0.0));
        assert!(cam.world_to_screen(rect, piv).is_some());
        assert!(cam.world_to_screen(rect, hdl).is_some());
    }
}
