use crate::*;

impl eframe::App for RailbenchApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let frame_start = self.perf_monitor.begin_frame();
        if self.pending_open_on_start {
            self.pending_open_on_start = false;
            self.open_layout_dialog();
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close_now && self.is_dirty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_confirm_action = Some(PendingConfirmAction::Quit);
        }
        let dirty_now = self.is_dirty();
        let title_state = (self.layout.name.clone(), dirty_now);
        if self.last_window_title_state.as_ref() != Some(&title_state) {
            let dot = if dirty_now { "• " } else { "" };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                "{dot}{} — {}",
                self.layout.name,
                version::full_app_title()
            )));
            self.last_window_title_state = Some(title_state);
        }
        self.tick_autosave();
        let should_sync_win = self
            .last_window_sync
            .map_or(true, |t| t.elapsed() >= std::time::Duration::from_secs(2));
        if should_sync_win {
            self.last_window_sync = Some(std::time::Instant::now());
            let (win_sz, win_pos, is_max, is_fullscreen) = ctx.input(|i| {
                (
                    i.viewport().inner_rect.map(|r| r.size()),
                    i.viewport().outer_rect.map(|r| r.min),
                    i.viewport().maximized.unwrap_or(false),
                    i.viewport().fullscreen.unwrap_or(false),
                )
            });
            let mut win_changed = self.app_settings.window_maximized != is_max;
            if let Some(sz) = win_sz {
                if !is_max && !is_fullscreen && sz.x >= 500.0 && sz.y >= 400.0 {
                    let prev_w = self.app_settings.window_width.unwrap_or(0.0);
                    let prev_h = self.app_settings.window_height.unwrap_or(0.0);
                    if (prev_w - sz.x).abs() > 8.0 || (prev_h - sz.y).abs() > 8.0 {
                        self.app_settings.window_width = Some(sz.x);
                        self.app_settings.window_height = Some(sz.y);
                        win_changed = true;
                    }
                }
            }
            if let Some(pos) = win_pos {
                if !is_max
                    && !is_fullscreen
                    && pos.x.is_finite()
                    && pos.y.is_finite()
                    && (-2000.0..=10000.0).contains(&pos.x)
                    && (-2000.0..=10000.0).contains(&pos.y)
                {
                    let prev_x = self.app_settings.window_x.unwrap_or(f32::NEG_INFINITY);
                    let prev_y = self.app_settings.window_y.unwrap_or(f32::NEG_INFINITY);
                    if (prev_x - pos.x).abs() > 8.0 || (prev_y - pos.y).abs() > 8.0 {
                        self.app_settings.window_x = Some(pos.x);
                        self.app_settings.window_y = Some(pos.y);
                        win_changed = true;
                    }
                }
            }
            if win_changed && self.persist_settings {
                self.app_settings.window_maximized = is_max;
                self.app_settings.save_to_disk();
            }
        }
        if !self.is_3d_mode && self.gpu_3d_resident {
            release_wgpu_3d_resources(frame.wgpu_render_state());
            self.gpu_3d_resident = false;
        }

        if self.recording_key_action.is_none() {
            let toggle_perf = ctx.input(|i| {
                self.app_settings.matches_action(KeyAction::TogglePerfMonitor, i)
            });
            if toggle_perf
                && (!ctx.wants_keyboard_input()
                    || self.app_settings.effective_key(KeyAction::TogglePerfMonitor).starts_with('F'))
            {
                self.perf_monitor.toggle();
                self.save_current_settings();
            }
            let (toggle_fullscreen, is_fullscreen) = ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::F11),
                    i.viewport().fullscreen.unwrap_or(false),
                )
            });
            if toggle_fullscreen && !ctx.wants_keyboard_input() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
            }
        }

        if let Some(incoming) = self.collab.poll() {
            self.layout = incoming;
            self.layout_version = self.layout_version.wrapping_add(1);
            if let Some(by) = self.collab.last_remote_update_by.take() {
                self.status_message = Some(format!("Collab: updated by {by}"));
            }
        }
        let chat_toasts: Vec<(String, String)> = self.collab.pending_chat_toasts.drain(..).collect();
        if !self.show_collab_dialog {
            if let Some((sender, text)) = chat_toasts.last() {
                let snippet: String = text.chars().take(48).collect();
                self.show_toast(format!("💬 {sender}: {snippet}"));
            }
        }
        if self.collab.is_active() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        let dropped_paths: Vec<std::path::PathBuf> = ctx.input(|i| {
            i.raw.dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped_paths {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext == "glb" || ext == "gltf" {
                self.import_custom_model_path(&path);
            } else if ext == "json" {
                if let Ok(raw) = std::fs::read_to_string(&path) {
                    let is_catalog_json = serde_json::from_str::<serde_json::Value>(&raw)
                        .ok()
                        .and_then(|v| {
                            let obj = v.as_object()?;
                            Some(obj.contains_key("entries") && !obj.contains_key("pieces"))
                        })
                        .unwrap_or(false);
                    if is_catalog_json {
                        match self.import_catalog_json_str(&raw) {
                            Ok(name) => self.show_toast(if self.lang == Lang::De {
                                format!("Katalog importiert: {name}")
                            } else {
                                format!("Imported catalog: {name}")
                            }),
                            Err(e) => self.status_message = Some(format!("Catalog import error: {e}")),
                        }
                    } else if let Some(s) = path.to_str() {
                        self.open_layout_path(s);
                    }
                }
            } else if matches!(ext.as_str(), "railbench" | "gleisplan" | "xml") {
                if let Some(s) = path.to_str() {
                    self.open_layout_path(s);
                }
            }
        }

        if self.app_settings.theme_mode == settings::ThemeMode::System {
            if let Some(sys_theme) = ctx.system_theme() {
                self.dark_mode = matches!(sys_theme, egui::Theme::Dark);
            }
        }

        if self.applied_theme_dark != Some(self.dark_mode) {
            theme::apply_railbench_theme(ctx, self.dark_mode);
            self.applied_theme_dark = Some(self.dark_mode);
        }


        self.handle_keyboard_shortcuts(ctx);


        // Keep cached power & 2-wire short-circuit simulation up to date (recomputes only when layout_version changes)
        if !self.layout.pieces.is_empty() || self.power_sim_enabled || self.show_power_dialog {
            let _ = self.get_or_update_power_sim();
        }
        if self.cad_tools.nem_check_enabled && self.cad_tools.nem_cache_version != self.layout_version {
            self.cad_tools.nem_violations =
                cad_tools::check_nem_clearances(&self.layout, &self.catalogs);
            self.cad_tools.nem_cache_version = self.layout_version;
        }

        // Advance active locomotives, road vehicles, uncoupler timers & animated turntable/transfer-table bridges
        let active_drive = (self.vehicle_sim.running && !self.vehicle_sim.vehicles.is_empty())
            || self.vehicle_sim.has_moving_bridges()
            || self.vehicle_sim.has_active_uncoupler_timers();
        if active_drive {
            let now = std::time::Instant::now();
            let dt = self
                .last_vehicle_step
                .map(|prev| (now - prev).as_secs_f64().clamp(0.0, 0.05))
                .unwrap_or(0.016);
            self.last_vehicle_step = Some(now);
            self.vehicle_sim.step_bridges(dt, &self.layout, &self.catalogs);
            if self.vehicle_sim.running && !self.vehicle_sim.vehicles.is_empty() {
                self.vehicle_sim.step(dt, &self.layout, &self.catalogs);
            }
            self.vehicle_sim.step_uncouplers(dt, &self.layout, &self.catalogs);
            if self.low_gpu_mode {
                ctx.request_repaint_after(std::time::Duration::from_millis(33));
            } else {
                ctx.request_repaint();
            }
        } else {
            self.last_vehicle_step = None;
        }

        draw_top_bars(ctx, self);
        draw_left_sidebar(ctx, self);

        let sim_for_canvas = self.power_sim_cache.as_ref().and_then(|(_, s)| {
            if self.power_sim_enabled || (s.is_two_wire && s.has_short_circuit) {
                Some(Arc::clone(s))
            } else {
                None
            }
        });

        let central_resp = egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let selection_scope = SelectionScope::from_sidebar_tab(self.sidebar_tab);
                if selection_scope != SelectionScope::All && !self.selected.is_empty() {
                    let pieces = &self.layout.pieces;
                    let models = self.layout.models.as_deref().unwrap_or(&[]);
                    let catalogs = &self.catalogs;
                    self.selected.retain(|id| {
                        pieces
                            .iter()
                            .any(|p| &p.id == id && selection_scope.allows_piece(catalogs, p))
                            || models
                                .iter()
                                .any(|m| &m.id == id && selection_scope.allows_model(m))
                    });
                }
                if self.is_3d_mode {
                    let now = std::time::Instant::now();
                    let dt = self
                        .last_3d_frame_time
                        .map(|prev| (now - prev).as_secs_f32().clamp(0.0, 0.1))
                        .unwrap_or(0.016);
                    self.last_3d_frame_time = Some(now);
                    if self.pending_fit_view {
                        self.camera_3d.fit_layout(&self.layout);
                        self.pending_fit_view = false;
                    }
                    self.ensure_mesh_3d();
                    self.gpu_3d_resident = true;
                    let sc_pts = self.current_short_circuit_points();
                    let vehicles_mesh = self
                        .vehicle_sim
                        .get_or_build_vehicles_mesh_3d(&self.layout.scale);
                    let edit_out = draw_viewport_3d(
                        ui,
                        &mut self.camera_3d,
                        &self.scene_3d_bundle,
                        vehicles_mesh,
                        viewer3d::Edit3DParams {
                            state: &mut self.edit_3d,
                            layout: &mut self.layout,
                            catalogs: &self.catalogs,
                            selected: &mut self.selected,
                            dragging_catalog: self.dragging_catalog_entry.as_ref(),
                            dragging_model: self.dragging_model_template.as_ref(),
                            terrain_brush: &mut self.terrain_brush,
                            snapping_enabled: self.snapping_enabled,
                            layer_protection: self.layer_protection,
                            rotation_step: self.rotation_step,
                            selection_scope,
                        },
                        &mut self.viewer3d_env,
                        &mut self.day_night_active,
                        &mut self.time_of_day,
                        &mut self.day_night_speed,
                        sc_pts.first().map(|p| Vec2 { x: p[0], y: p[2] }),
                        self.dark_mode,
                        self.low_gpu_mode,
                        self.lang,
                        dt,
                    );
                    if edit_out.push_undo {
                        self.push_undo();
                    }
                    if let Some((drop_entry, drop_pt, snap_r)) = edit_out.drop_catalog_at {
                        self.dragging_catalog_entry = None;
                        self.place_catalog_entry_at(&drop_entry, drop_pt, snap_r);
                    }
                    if let Some((mut drop_model, drop_pt)) = edit_out.drop_model_at {
                        self.dragging_model_template = None;
                        drop_model.transform.x = drop_pt.x;
                        drop_model.transform.y = drop_pt.y;
                        self.place_model_instance(drop_model);
                    }
                    if let Some((pid, cidx)) = edit_out.quick_add_at {
                        self.append_track_to_connector(&pid, cidx, None);
                    }
                    if let Some(delta_deg) = edit_out.rotate_delta_deg {
                        self.rotate_selected(delta_deg);
                    }
                    if edit_out.layout_changed {
                        self.mark_layout_dirty();
                    }
                    if self.day_night_active {
                        if self.low_gpu_mode {
                            ctx.request_repaint_after(std::time::Duration::from_millis(33));
                        } else {
                            ctx.request_repaint();
                        }
                    }
                } else {
                    self.last_3d_frame_time = None;
                    if self.pending_fit_view {
                        let rect = ui.available_rect_before_wrap();
                        self.camera.fit_layout(rect, &self.layout);
                        self.pending_fit_view = false;
                    }
                    if let Some(fp) = self.pending_focus_point.take() {
                        let rect = ui.available_rect_before_wrap();
                        self.camera.zoom = self.camera.zoom.max(0.75);
                        self.camera.offset = egui::vec2(
                            rect.width() * 0.5 - (fp.x as f32) * self.camera.zoom,
                            rect.height() * 0.5 - (fp.y as f32) * self.camera.zoom,
                        );
                    }

                    let ramp_overlay = if self.show_height_dialog && (self.height_tab == 1 || self.height_tab == 2) {
                        if self.height_tab != 1 {
                            self.ramp_sel.pick_mode = None;
                        }
                        if self.height_tab != 2 {
                            self.gen_pick_mode = false;
                            self.gen_preview_pieces.clear();
                        }
                        let gen_start_h = self
                            .gen_anchor_piece_id
                            .as_deref()
                            .and_then(|aid| {
                                let ci = self.gen_conn_idx.unwrap_or(0);
                                crate::ramp_tools::piece_world_height_mm(&self.layout, aid, ci != 0)
                            })
                            .unwrap_or(0.0);
                        Some(canvas2d::HeightPanelOverlay {
                            tab: self.height_tab,
                            ramp_sel: &mut self.ramp_sel,
                            ramp_start_h: &mut self.ramp_start_h,
                            ramp_end_h: &mut self.ramp_end_h,
                            gen_pick_mode: &mut self.gen_pick_mode,
                            gen_anchor_piece_id: &mut self.gen_anchor_piece_id,
                            gen_conn_idx: &mut self.gen_conn_idx,
                            gen_preview_pieces: &self.gen_preview_pieces,
                            gen_start_h,
                            gen_target_h: self.gen_target_h,
                        })
                    } else {
                        self.ramp_sel.pick_mode = None;
                        self.gen_pick_mode = false;
                        self.gen_preview_pieces.clear();
                        None
                    };
                    let collab_users_snapshot: Vec<crate::collab::CollabUser> =
                        if self.collab.connected {
                            self.collab.users.clone()
                        } else {
                            Vec::new()
                        };
                    let res = draw_canvas_2d(
                        ui,
                        &mut self.layout,
                        self.layout_version,
                        &mut self.canvas_2d_cache,
                        &self.catalogs,
                        &mut self.camera,
                        &mut self.selected,
                        &mut self.dragging_active,
                        &mut self.drag_last_world,
                        &mut self.dragging_flex_handle,
                        &mut self.marquee_start,
                        &mut self.measure_points,
                        &mut self.measure_locked,
                        self.dragging_catalog_entry.as_ref(),
                        self.dragging_model_template.as_ref(),
                        &collab_users_snapshot,
                        self.dark_mode,
                        self.color_mode,
                        self.show_labels,
                        self.show_connectors,
                        self.show_rulers,
                        self.snapping_enabled,
                        self.layer_protection,
                        self.power_sim_enabled,
                        sim_for_canvas.as_deref(),
                        self.measure_mode,
                        self.rotation_step,
                        &mut self.terrain_brush,
                        self.lang == Lang::De,
                        &mut self.next_piece_seq,
                        &mut self.active_board_id,
                        &mut self.dragging_board,
                        &mut self.board_snap_guides,
                        self.show_wood_plates,
                        self.sidebar_tab == SidebarTab::Board,
                        selection_scope,
                        &mut self.sketcher,
                        &mut self.dragging_model_rot,
                        &mut self.dragging_track_rot,
                        ramp_overlay,
                        &self.vehicle_sim,
                        &mut self.cad_tools,
                    );
                    if self.collab.connected {
                        if let Some(hw) = res.hover_world {
                            let should_send = match self.last_collab_cursor_sent {
                                None => true,
                                Some((ts, prev)) => {
                                    ts.elapsed() >= std::time::Duration::from_millis(50)
                                        && (hw.x - prev.x).hypot(hw.y - prev.y) > 1.0
                                }
                            };
                            if should_send {
                                self.collab.send_cursor(hw.x, hw.y);
                                self.last_collab_cursor_sent = Some((std::time::Instant::now(), hw));
                            }
                        }
                        let mut sel_vec: Vec<String> = self.selected.iter().cloned().collect();
                        sel_vec.sort();
                        if sel_vec != self.last_collab_selection_sent {
                            self.collab.send_selection(&sel_vec);
                            self.last_collab_selection_sent = sel_vec;
                        }
                    } else {
                        self.last_collab_cursor_sent = None;
                        self.last_collab_selection_sent.clear();
                    }
                    if let Some((drop_world, snap_r)) = res.drop_catalog_at {
                        if let Some(entry) = self.dragging_catalog_entry.take() {
                            self.place_catalog_entry_at(&entry, drop_world, snap_r);
                        }
                    }
                    if let Some(drop_wp) = res.drop_model_at {
                        if let Some(mut model) = self.dragging_model_template.take() {
                            model.transform.x = drop_wp.x;
                            model.transform.y = drop_wp.y;
                            self.place_model_instance(model);
                        }
                    }
                    if let Some((pid, cidx)) = res.quick_add_at {
                        self.append_track_to_connector(&pid, cidx, None);
                    }
                    if let Some(pid) = res.trigger_uncoupler_piece {
                        self.vehicle_sim.trigger_uncoupler(&pid, &self.layout, &self.catalogs);
                    }
                    if let Some(pid) = res.toggle_uncoupler_armed {
                        self.vehicle_sim.toggle_uncoupler_armed(&pid);
                    }
                    if res.push_undo {
                        self.push_undo();
                    }
                    if res.changed {
                        self.mark_layout_dirty();
                    }
                    if let Some(delta) = res.rotate_delta {
                        self.rotate_selected(delta);
                    }
                    if res.copy_selected {
                        self.copy_selected();
                    }
                    if res.paste_clipboard {
                        self.paste_clipboard(None);
                    }
                    if res.duplicate_selected {
                        self.duplicate_selected();
                    }
                    if res.flip_selected {
                        self.flip_selected();
                    }
                    if res.toggle_group {
                        self.toggle_group_selected();
                    }
                    if res.select_all {
                        self.select_all();
                    }
                    if res.fit_view {
                        self.pending_fit_view = true;
                    }
                    if res.close_track_gap {
                        self.run_close_gap();
                    }
                    if res.generate_parallel_tracks {
                        self.run_parallel_tracks();
                    }
                    if res.open_height_dialog {
                        self.height_tab = 1;
                        self.show_height_dialog = true;
                    }
                    if res.auto_ramp {
                        self.apply_auto_ramp_one_shot();
                    }
                    if let Some(pid) = res.open_replace_split {
                        self.replace_track_search.clear();
                        self.show_replace_track_piece_id = Some(pid);
                    }
                    if let Some(pid) = res.reset_flex {
                        self.push_undo();
                        if let Some(pc) = self.layout.pieces.iter_mut().find(|p| p.id == pid) {
                            crate::catalog::reset_flex_track_to_original(pc, &self.catalogs);
                        }
                        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
                        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
                        self.mark_layout_dirty();
                    }
                    if let Some(pid) = res.open_track_props {
                        self.show_track_props_piece_id = Some(pid);
                    }
                    if res.select_connected {
                        self.select_connected_tracks();
                    }
                    if res.select_layer {
                        self.select_layer_tracks();
                    }
                    if let Some(lid) = res.move_to_layer {
                        self.move_selected_to_layer(&lid);
                    }
                    if res.toggle_feeder {
                        self.toggle_selected_feeder();
                    }
                    if res.toggle_isolator {
                        self.toggle_selected_isolator();
                    }
                    if res.toggle_catenary {
                        self.toggle_selected_catenary();
                    }
                    if res.toggle_catenary_feeder {
                        self.toggle_selected_catenary_feeder();
                    }
                    if res.toggle_catenary_isolator {
                        self.toggle_selected_catenary_isolator();
                    }
                    if res.toggle_lantern {
                        self.toggle_selected_lantern();
                    }
                    if res.toggle_decoder {
                        self.toggle_selected_decoder();
                    }
                    if res.toggle_signal {
                        self.toggle_selected_signal();
                    }
                    if res.cycle_signal_side {
                        self.cycle_selected_signal_side();
                    }
                    if res.cycle_signal_direction {
                        self.cycle_selected_signal_direction();
                    }
                    if res.toggle_bridge {
                        self.toggle_selected_bridge();
                    }
                    if res.toggle_tunnel {
                        self.toggle_selected_tunnel();
                    }
                    if res.delete_selected {
                        self.delete_selected();
                    }
                }
            });
        let viewport_rect = central_resp.response.rect;

        // Floating layers panel & dialogs overlay on top of the full-width 2D/3D viewport
        draw_right_sidebar(ctx, self, viewport_rect);
        draw_dialogs(ctx, self);

        // Separate OS Window: Unified Drive, 3D Cab View & Signal Box (Stellwerk)
        if self.show_cab_window {
            ui_panels::draw_cab_view_window(ctx, self);
        }

        // Floating drag badge while dragging a catalog piece or 3D model
        if let Some(entry) = &self.dragging_catalog_entry {
            if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("catalog_drag_ghost"),
                ));
                let pill = egui::Rect::from_min_size(pos + egui::vec2(12.0, 12.0), egui::vec2(140.0, 26.0));
                painter.rect_filled(pill, 7.0, egui::Color32::from_rgba_unmultiplied(15, 23, 42, 230));
                painter.rect_stroke(
                    pill,
                    7.0,
                    egui::Stroke::new(1.2_f32, egui::Color32::from_rgb(56, 189, 248)),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    pill.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("+ {}", entry.id),
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            }
            // If released anywhere outside the canvas, cancel the drag cleanly
            if ctx.input(|i| i.pointer.primary_released() || !i.pointer.primary_down()) {
                self.dragging_catalog_entry = None;
            }
        }
        if let Some(model) = &self.dragging_model_template {
            if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("model_drag_ghost"),
                ));
                let w = (model.name.len() as f32 * 6.8 + 26.0).clamp(110.0, 220.0);
                let pill = egui::Rect::from_min_size(pos + egui::vec2(12.0, 12.0), egui::vec2(w, 26.0));
                painter.rect_filled(pill, 7.0, egui::Color32::from_rgba_unmultiplied(15, 23, 42, 230));
                painter.rect_stroke(
                    pill,
                    7.0,
                    egui::Stroke::new(1.2_f32, egui::Color32::from_rgb(16, 185, 129)),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    pill.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("+ {}", model.name),
                    egui::FontId::proportional(11.5),
                    egui::Color32::WHITE,
                );
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            }
            if ctx.input(|i| i.pointer.primary_released() || !i.pointer.primary_down()) {
                self.dragging_model_template = None;
            }
        }

        // Floating Interactive Measure HUD when Measure Tool is active in 2D
        if !self.is_3d_mode && self.measure_mode {
            let is_de = self.lang == Lang::De;
            let p = theme::palette(self.dark_mode);
            let scale_den = match self.layout.scale.trim().to_ascii_uppercase().as_str() {
                "Z" => 220.0,
                "N" => 160.0,
                "TT" => 120.0,
                "S" => 64.0,
                "0" | "O" => 45.0,
                "1" => 32.0,
                "G" | "2" | "IIM" | "2M" => 22.5,
                _ => 87.0,
            };
            egui::Area::new(egui::Id::new("railbench_measure_hud"))
                .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 96.0))
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(p.bg_card)
                        .corner_radius(egui::CornerRadius::same(10))
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .stroke(egui::Stroke::new(1.3_f32, egui::Color32::from_rgb(245, 158, 11)))
                        .show(ui, |ui| {
                            if let Some((mut a, mut b)) = self.measure_points {
                                let dx = b.x - a.x;
                                let dy = b.y - a.y;
                                let dist_mm = dx.hypot(dy);
                                let proto_m = (dist_mm / 1000.0) * scale_den;
                                let proto_str = if proto_m >= 1000.0 {
                                    format!("{:.2} km", proto_m / 1000.0)
                                } else {
                                    format!("{proto_m:.1} m")
                                };
                                let angle_deg = dy.atan2(dx).to_degrees();
                                ui.horizontal(|ui| {
                                    ui.strong(format!("📏 {dist_mm:.1} mm ({:.2} cm)", dist_mm / 10.0));
                                    ui.separator();
                                    let den_str = if (scale_den - scale_den.round()).abs() > 0.01 {
                                        format!("{scale_den:.1}")
                                    } else {
                                        format!("{scale_den:.0}")
                                    };
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} 1:{den_str} ≈ {proto_str}",
                                            self.layout.scale
                                        ))
                                        .color(p.primary)
                                        .strong(),
                                    );
                                    ui.separator();
                                    ui.label(format!("ΔX {dx:.1}  ΔY {dy:.1} mm · {angle_deg:.1}°"));
                                    if ui.small_button(if is_de { "Neu" } else { "New" }).clicked() {
                                        self.measure_points = None;
                                        self.measure_locked = false;
                                    }
                                    if ui.small_button("×").clicked() {
                                        self.measure_mode = false;
                                        self.measure_points = None;
                                        self.measure_locked = false;
                                    }
                                });
                                ui.add_space(3.0);
                                ui.horizontal(|ui| {
                                    let mut edited = false;
                                    ui.label("A:");
                                    edited |= ui.add(egui::DragValue::new(&mut a.x).speed(1.0).suffix(" mm")).changed();
                                    edited |= ui.add(egui::DragValue::new(&mut a.y).speed(1.0).suffix(" mm")).changed();
                                    ui.separator();
                                    ui.label("B:");
                                    edited |= ui.add(egui::DragValue::new(&mut b.x).speed(1.0).suffix(" mm")).changed();
                                    edited |= ui.add(egui::DragValue::new(&mut b.y).speed(1.0).suffix(" mm")).changed();
                                    if edited {
                                        self.measure_points = Some((a, b));
                                        self.measure_locked = true;
                                    }
                                    ui.weak(if is_de { "(Alt = Verbinder fangen)" } else { "(Alt = snap connector)" });
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    ui.label(if is_de {
                                        "📏 Punkt A -> B klicken oder ziehen · Alt = Gleisverbinder fangen"
                                    } else {
                                        "📏 Click or drag A -> B to measure · Hold Alt to snap to connectors"
                                    });
                                    if ui.small_button("×").clicked() {
                                        self.measure_mode = false;
                                        self.measure_locked = false;
                                    }
                                });
                            }
                        });
                });
        }

        // Floating selection bar at bottom-center of canvas in both 2D and 3D views (skip during 3D heightfield sculpt HUD)
        let terrain_3d_sculpt_active = self.is_3d_mode
            && self.terrain_brush.active
            && self.terrain_brush.is_heightfield_mode();
        if !terrain_3d_sculpt_active {
            draw_floating_selection_bar(ctx, self);
        }

        // F3 Hardware & Performance Monitor overlay
        if self.perf_monitor.visible {
            if let Some(stress_count) = self.perf_monitor.draw_overlay(
                ctx,
                &self.layout,
                self.scene_3d_bundle.as_ref(),
                self.is_3d_mode,
                self.dark_mode,
                self.lang,
            ) {
                self.generate_stress_tracks(stress_count);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        // Floating toast notification pill (auto-dismisses after 2.2s)
        if let Some((ref msg, started)) = self.toast_message {
            if started.elapsed() < std::time::Duration::from_millis(2200) {
                egui::Area::new(egui::Id::new("railbench_toast_pill"))
                    .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -68.0))
                    .show(ctx, |ui| {
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgba_unmultiplied(15, 23, 42, 238))
                            .corner_radius(egui::CornerRadius::same(16))
                            .inner_margin(egui::Margin::symmetric(14, 7))
                            .stroke(egui::Stroke::new(1.2_f32, egui::Color32::from_rgb(56, 189, 248)))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(msg).strong().size(12.5).color(egui::Color32::WHITE));
                            });
                    });
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            } else {
                self.toast_message = None;
            }
        }

        // Frameless window edge/corner resize handles (for Wayland & X11)
        handle_window_resize_borders(ctx);
        self.perf_monitor.end_frame(frame_start);
    }
}

