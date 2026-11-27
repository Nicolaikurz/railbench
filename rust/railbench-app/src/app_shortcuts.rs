use crate::*;

impl RailbenchApp {
    pub(crate) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // Dynamic customizable keyboard shortcuts (disabled while recording a new shortcut or typing text)
        if self.recording_key_action.is_none() && !ctx.wants_keyboard_input() {
            let (
                do_undo,
                do_redo,
                do_save_as,
                do_save,
                do_open,
                do_new_plan,
                do_export_csv,
                do_print_plan,
                do_copy,
                do_paste,
                do_dup,
                do_sel_all,
                do_deselect,
                toggle_sel_lock,
                do_fit,
                zoom_in,
                zoom_out,
                pan_up,
                pan_down,
                pan_left,
                pan_right,
                pan_fast,
                rot_delta,
                do_flip,
                do_group,
                do_split_replace,
                do_track_props,
                do_delete,
                toggle_3d,
                toggle_rulers,
                toggle_labels,
                toggle_snapping,
                toggle_power_sim,
                shift_m,
                toggle_measure,
                cycle_color,
                toggle_layers_sidebar,
                toggle_feeder,
                toggle_cat_feed,
                toggle_iso,
                toggle_cat_iso,
                toggle_cat,
                toggle_bridge,
                toggle_tunnel,
                toggle_lantern,
                toggle_decoder,
                toggle_signal,
                open_help,
            ) = ctx.input(|i| {
                let s = &self.app_settings;
                let copy = i.events.iter().any(|e| matches!(e, egui::Event::Copy))
                    || s.matches_action(KeyAction::Copy, i);
                let paste = i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)))
                    || s.matches_action(KeyAction::Paste, i);
                let rot_cw = s.matches_action(KeyAction::RotateCw, i);
                let rot_ccw = s.matches_action(KeyAction::RotateCcw, i);
                let rot = if rot_ccw {
                    Some(-1.0)
                } else if rot_cw {
                    Some(1.0)
                } else {
                    None
                };
                let del_pressed = s.matches_action(KeyAction::DeleteSelected, i)
                    && !(self.sketcher.active && i.key_pressed(egui::Key::Backspace));
                let shift_m_pressed = s.matches_action(KeyAction::AutoRamp, i);
                (
                    s.matches_action(KeyAction::Undo, i),
                    s.matches_action(KeyAction::Redo, i),
                    s.matches_action(KeyAction::SavePlanAs, i),
                    s.matches_action(KeyAction::Save, i),
                    s.matches_action(KeyAction::Open, i),
                    s.matches_action(KeyAction::NewPlan, i),
                    s.matches_action(KeyAction::ExportCsv, i),
                    s.matches_action(KeyAction::PrintPlan, i),
                    copy,
                    paste,
                    s.matches_action(KeyAction::Duplicate, i),
                    s.matches_action(KeyAction::SelectAll, i),
                    s.matches_action(KeyAction::DeselectAll, i),
                    s.matches_action(KeyAction::ToggleSelectionLock, i),
                    s.matches_action(KeyAction::FitView, i),
                    s.matches_action(KeyAction::ZoomIn, i),
                    s.matches_action(KeyAction::ZoomOut, i),
                    s.matches_action(KeyAction::PanUp, i),
                    s.matches_action(KeyAction::PanDown, i),
                    s.matches_action(KeyAction::PanLeft, i),
                    s.matches_action(KeyAction::PanRight, i),
                    i.modifiers.shift,
                    rot,
                    s.matches_action(KeyAction::FlipSelected, i),
                    s.matches_action(KeyAction::ToggleGroup, i),
                    s.matches_action(KeyAction::SplitReplaceTrack, i),
                    s.matches_action(KeyAction::TrackProperties, i),
                    del_pressed,
                    s.matches_action(KeyAction::Toggle3D, i),
                    s.matches_action(KeyAction::ToggleRulers, i),
                    s.matches_action(KeyAction::ToggleLabels, i),
                    s.matches_action(KeyAction::ToggleSnapping, i),
                    s.matches_action(KeyAction::TogglePowerSim, i),
                    shift_m_pressed,
                    s.matches_action(KeyAction::ToggleMeasure, i),
                    s.matches_action(KeyAction::CycleColorMode, i),
                    s.matches_action(KeyAction::ToggleLayersSidebar, i),
                    s.matches_action(KeyAction::ToggleFeeder, i),
                    s.matches_action(KeyAction::ToggleCatenaryFeeder, i),
                    s.matches_action(KeyAction::ToggleIsolator, i),
                    s.matches_action(KeyAction::ToggleCatenaryIsolator, i),
                    s.matches_action(KeyAction::ToggleCatenary, i),
                    s.matches_action(KeyAction::ToggleBridge, i),
                    s.matches_action(KeyAction::ToggleTunnel, i),
                    s.matches_action(KeyAction::ToggleLantern, i),
                    s.matches_action(KeyAction::ToggleDecoder, i),
                    s.matches_action(KeyAction::ToggleSignal, i),
                    s.matches_action(KeyAction::OpenHelp, i),
                )
            });

            if do_undo { self.undo(); }
            if do_redo { self.redo(); }
            if do_save_as {
                self.save_layout_as_dialog();
            } else if do_save {
                self.save_layout_dialog();
            }
            if do_open { self.request_open_layout_dialog(); }
            if do_new_plan { self.request_new_layout(); }
            if do_export_csv { self.export_bom_csv_dialog(); }
            if do_print_plan { self.show_export_dialog = true; }
            if do_copy { self.copy_selected(); }
            if do_paste { self.paste_clipboard(None); }
            if do_dup { self.duplicate_selected(); }
            if do_sel_all { self.select_all(); }
            if toggle_sel_lock {
                self.layer_protection = !self.layer_protection;
                self.save_current_settings();
            }
            if do_fit { self.pending_fit_view = true; }
            if zoom_in {
                if self.is_3d_mode {
                    self.camera_3d.distance = (self.camera_3d.distance / 1.15).clamp(60.0, 25000.0);
                } else {
                    let anchor = ctx.screen_rect().center();
                    let wx = (anchor.x - self.camera.offset.x) / self.camera.zoom;
                    let wy = (anchor.y - self.camera.offset.y) / self.camera.zoom;
                    self.camera.zoom = (self.camera.zoom * 1.15).clamp(0.04, 15.0);
                    self.camera.offset.x = anchor.x - wx * self.camera.zoom;
                    self.camera.offset.y = anchor.y - wy * self.camera.zoom;
                }
            }
            if zoom_out {
                if self.is_3d_mode {
                    self.camera_3d.distance = (self.camera_3d.distance * 1.15).clamp(60.0, 25000.0);
                } else {
                    let anchor = ctx.screen_rect().center();
                    let wx = (anchor.x - self.camera.offset.x) / self.camera.zoom;
                    let wy = (anchor.y - self.camera.offset.y) / self.camera.zoom;
                    self.camera.zoom = (self.camera.zoom / 1.15).clamp(0.04, 15.0);
                    self.camera.offset.x = anchor.x - wx * self.camera.zoom;
                    self.camera.offset.y = anchor.y - wy * self.camera.zoom;
                }
            }
            let mut pan_dx = 0.0_f32;
            let mut pan_dy = 0.0_f32;
            let pan_step = if pan_fast { 180.0_f32 } else { 60.0_f32 };
            if pan_up { pan_dy += pan_step; }
            if pan_down { pan_dy -= pan_step; }
            if pan_left { pan_dx += pan_step; }
            if pan_right { pan_dx -= pan_step; }
            if pan_dx != 0.0 || pan_dy != 0.0 {
                if self.is_3d_mode {
                    let speed = self.camera_3d.distance * 0.0012;
                    let rx = self.camera_3d.yaw.cos();
                    let rz = -self.camera_3d.yaw.sin();
                    let fx = -self.camera_3d.yaw.sin();
                    let fz = -self.camera_3d.yaw.cos();
                    self.camera_3d.target[0] -= (rx * pan_dx - fx * pan_dy) * speed;
                    self.camera_3d.target[2] -= (rz * pan_dx - fz * pan_dy) * speed;
                } else {
                    self.camera.offset.x += pan_dx;
                    self.camera.offset.y += pan_dy;
                }
            }
            if let Some(dir) = rot_delta {
                self.rotate_selected(dir * self.rotation_step);
            }
            if do_flip {
                self.flip_selected();
            }
            if do_group {
                self.toggle_group_selected();
            }
            if do_split_replace {
                if self.selected.len() == 1 {
                    if let Some(pid) = self.layout.pieces.iter().find(|p| self.selected.contains(&p.id)).map(|p| p.id.clone()) {
                        self.replace_track_search.clear();
                        self.show_replace_track_piece_id = Some(pid);
                    }
                }
            }
            if do_track_props {
                if self.selected.len() == 1 {
                    if let Some(pid) = self.layout.pieces.iter().find(|p| self.selected.contains(&p.id)).map(|p| p.id.clone()) {
                        self.show_track_props_piece_id = Some(pid);
                    }
                }
            }
            if do_delete { self.delete_selected(); }
            if toggle_3d { self.toggle_3d(); }
            if toggle_rulers {
                self.show_rulers = !self.show_rulers;
                self.save_current_settings();
            }
            if toggle_labels {
                self.show_labels = !self.show_labels;
                self.save_current_settings();
            }
            if toggle_snapping {
                self.snapping_enabled = !self.snapping_enabled;
                self.save_current_settings();
            }
            if toggle_power_sim {
                self.power_sim_enabled = !self.power_sim_enabled;
                self.mark_layout_dirty();
            }
            if shift_m {
                self.apply_auto_ramp_one_shot();
            } else if toggle_measure {
                if !self.selected.is_empty() && !self.measure_mode {
                    self.show_height_dialog = true;
                } else {
                    self.measure_mode = !self.measure_mode;
                    if !self.measure_mode {
                        self.measure_points = None;
                        self.measure_locked = false;
                    }
                }
            }
            if cycle_color {
                self.color_mode = match self.color_mode {
                    ColorMode::Types => ColorMode::Layers,
                    ColorMode::Layers => ColorMode::Off,
                    ColorMode::Off => ColorMode::Types,
                };
                self.save_current_settings();
            }
            if toggle_layers_sidebar {
                self.show_right_sidebar = !self.show_right_sidebar;
                self.save_current_settings();
            }
            if toggle_feeder { self.toggle_selected_feeder(); }
            if toggle_cat_feed { self.toggle_selected_catenary_feeder(); }
            if toggle_iso { self.toggle_selected_isolator(); }
            if toggle_cat_iso { self.toggle_selected_catenary_isolator(); }
            if toggle_cat { self.toggle_selected_catenary(); }
            if toggle_bridge { self.toggle_selected_bridge(); }
            if toggle_tunnel { self.toggle_selected_tunnel(); }
            if toggle_lantern { self.toggle_selected_lantern(); }
            if toggle_decoder { self.toggle_selected_decoder(); }
            if toggle_signal { self.toggle_selected_signal(); }
            if open_help { self.show_help_dialog = true; }
            if do_deselect {
                if self.show_help_dialog {
                    self.show_help_dialog = false;
                } else if self.measure_mode {
                    if self.measure_points.is_some() {
                        self.measure_points = None;
                        self.measure_locked = false;
                    } else {
                        self.measure_mode = false;
                        self.measure_locked = false;
                    }
                } else if self.terrain_brush.active
                    || self.dragging_catalog_entry.is_some()
                    || self.dragging_model_template.is_some()
                    || self.dragging_active
                    || self.marquee_start.is_some()
                    || self.dragging_flex_handle.is_some()
                    || self.sketcher.active
                    || self.dragging_board.is_some()
                    || self.dragging_model_rot.is_some()
                    || self.dragging_track_rot.is_some()
                    || self.ramp_sel.pick_mode.is_some()
                    || self.gen_pick_mode
                {
                    self.terrain_brush.set_active(false, None, None);
                    self.dragging_catalog_entry = None;
                    self.dragging_model_template = None;
                    self.dragging_active = false;
                    self.drag_last_world = None;
                    self.marquee_start = None;
                    self.dragging_flex_handle = None;
                    self.sketcher.stop();
                    self.dragging_board = None;
                    self.dragging_model_rot = None;
                    self.dragging_track_rot = None;
                    self.board_snap_guides.clear();
                    self.ramp_sel.pick_mode = None;
                    self.gen_pick_mode = false;
                } else if self.is_3d_mode {
                    if self.edit_3d.enabled && !self.selected.is_empty() {
                        self.selected.clear();
                    } else {
                        self.toggle_3d();
                    }
                } else {
                    self.selected.clear();
                }
            }
        }
    }
}
