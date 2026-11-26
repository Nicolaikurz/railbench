use crate::*;

pub(crate) const MAX_UNDO_STEPS: usize = 60;
pub(crate) const MIN_UNDO_STEPS: usize = 8;
pub(crate) const UNDO_MEMORY_BUDGET_BYTES: usize = 48 * 1024 * 1024;

pub(crate) fn estimate_layout_ram_bytes(layout: &Layout) -> usize {
    let pieces_bytes = layout.pieces.len() * 640
        + layout
            .pieces
            .iter()
            .map(|p| p.connectors.len() * 48)
            .sum::<usize>();
    // Note: `Model3D::file_data` is a shared `Arc<str>` across undo snapshots, so only count per-model struct overhead.
    let models_bytes = layout.models.as_ref().map_or(0, |ms| {
        ms.len() * 320
            + ms.iter()
                .filter_map(|m| m.mountain_path.as_ref())
                .map(|path| path.len() * 32)
                .sum::<usize>()
    });
    let layers_bytes = layout.layers.as_ref().map_or(0, |ls| ls.len() * 256);
    let boards_bytes = layout.baseboards.as_ref().map_or(0, |bs| bs.len() * 256);
    pieces_bytes + models_bytes + layers_bytes + boards_bytes + 512
}

pub(crate) fn dedup_model_file_data(models: &mut [Model3D], library: &[Model3D]) {
    let mut seen: HashSet<Arc<str>> = HashSet::new();
    for m in library {
        if let Some(ref fd) = m.file_data {
            seen.insert(Arc::clone(fd));
        }
    }
    for m in models {
        if let Some(ref fd) = m.file_data {
            if let Some(existing) = seen.get(fd) {
                m.file_data = Some(Arc::clone(existing));
            } else {
                seen.insert(Arc::clone(fd));
            }
        }
    }
}


pub(crate) fn handle_window_resize_borders(ctx: &egui::Context) {
    let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    if is_max {
        return;
    }
    let screen = ctx.screen_rect();
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let margin = 5.0_f32;
    let left = pos.x <= screen.min.x + margin;
    let right = pos.x >= screen.max.x - margin;
    let top = pos.y <= screen.min.y + margin;
    let bottom = pos.y >= screen.max.y - margin;

    let dir = match (left, right, top, bottom) {
        (true, _, true, _) => Some((egui::ResizeDirection::NorthWest, egui::CursorIcon::ResizeNwSe)),
        (_, true, true, _) => Some((egui::ResizeDirection::NorthEast, egui::CursorIcon::ResizeNeSw)),
        (true, _, _, true) => Some((egui::ResizeDirection::SouthWest, egui::CursorIcon::ResizeNeSw)),
        (_, true, _, true) => Some((egui::ResizeDirection::SouthEast, egui::CursorIcon::ResizeNwSe)),
        (true, _, _, _) => Some((egui::ResizeDirection::West, egui::CursorIcon::ResizeHorizontal)),
        (_, true, _, _) => Some((egui::ResizeDirection::East, egui::CursorIcon::ResizeHorizontal)),
        (_, _, true, _) => Some((egui::ResizeDirection::North, egui::CursorIcon::ResizeVertical)),
        (_, _, _, true) => Some((egui::ResizeDirection::South, egui::CursorIcon::ResizeVertical)),
        _ => None,
    };

    if let Some((resize_dir, cursor)) = dir {
        ctx.set_cursor_icon(cursor);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(resize_dir));
        }
    }
}

pub(crate) fn make_empty_layout() -> Layout {
    let default_board = BaseboardConfig {
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
    };
    Layout {
        name: "Neue Anlage".into(),
        scale: "H0".into(),
        pieces: Vec::new(),
        models: Some(Vec::new()),
        feed_voltage: Some(16.0),
        catenary_feed_voltage: None,
        catenary_circuit_mode: None,
        loco_current_ma: Some(450.0),
        loco_count: Some(1),
        train_current_amps: None,
        turnout_lantern_current_ma: Some(20.0),
        turnout_decoder_current_ma: Some(15.0),
        layers: Some(vec![default_layer()]),
        active_layer_id: Some("layer-1".into()),
        baseboard: Some(default_board.clone()),
        baseboards: Some(vec![default_board]),
        terrain_field: None,
    }
}

impl RailbenchApp {
    pub fn apply_auto_ramp_one_shot(&mut self) {
        if let Some((chain, start_h, end_h)) =
            ramp_tools::calculate_auto_ramp(&self.layout, &self.selected, &self.catalogs)
        {
            if (end_h - start_h).abs() >= 0.2 {
                self.push_undo();
                match ramp_tools::apply_auto_ramp_for_chain(
                    &mut self.layout,
                    &self.catalogs,
                    &chain,
                    start_h,
                    end_h,
                    self.ramp_profile,
                ) {
                    Ok(sum) => {
                        self.mark_layout_dirty();
                        let msg = if self.lang == Lang::De {
                            format!(
                                "Auto-Rampe: {:.0} -> {:.0} mm ueber {} Gleise ({:.1}%)",
                                start_h, end_h, sum.piece_count, sum.max_gradient_pct
                            )
                        } else {
                            format!(
                                "Auto-Ramp: {:.0} -> {:.0} mm across {} pieces ({:.1}%)",
                                start_h, end_h, sum.piece_count, sum.max_gradient_pct
                            )
                        };
                        self.show_toast(msg);
                        return;
                    }
                    Err(e) => self.status_message = Some(e),
                }
            }
        }
        self.height_tab = 1;
        self.show_height_dialog = true;
    }

    pub fn run_close_gap(&mut self) {
        let mut temp_layout = self.layout.clone();
        let mut temp_seq = self.next_piece_seq;
        match cad_tools::close_gap_between_selected(
            &mut temp_layout,
            &self.catalogs,
            &self.selected,
            &mut temp_seq,
            self.selected_catalog_idx,
        ) {
            Ok((added, msg)) => {
                self.push_undo();
                self.layout = temp_layout;
                self.next_piece_seq = temp_seq;
                self.selected.clear();
                for id in added {
                    self.selected.insert(id);
                }
                self.status_message = Some(msg);
                self.mark_layout_dirty();
            }
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    pub fn run_parallel_tracks(&mut self) {
        if let Some(first_id) = self.selected.iter().next() {
            if let Some(p) = self.layout.pieces.iter().find(|pc| &pc.id == first_id) {
                let is_default = (self.cad_tools.parallel_spacing_mm - 64.3).abs() < 0.05;
                if is_default {
                    self.cad_tools.parallel_spacing_mm =
                        cad_tools::CadToolsState::default_spacing_for_catalog(
                            &self.layout.scale,
                            &p.catalog_id,
                        );
                }
            }
        }
        let mut temp_layout = self.layout.clone();
        let mut temp_seq = self.next_piece_seq;
        match cad_tools::generate_parallel_tracks(
            &mut temp_layout,
            &self.catalogs,
            &self.selected,
            self.cad_tools.parallel_spacing_mm,
            self.cad_tools.parallel_side,
            &mut temp_seq,
        ) {
            Ok(added) => {
                let cnt = added.len();
                self.push_undo();
                self.layout = temp_layout;
                self.next_piece_seq = temp_seq;
                self.selected.clear();
                for id in added {
                    self.selected.insert(id);
                }
                self.status_message = Some(format!(
                    "Generated {cnt} parallel track piece(s) ({:.1} mm)",
                    self.cad_tools.parallel_spacing_mm
                ));
                self.mark_layout_dirty();
            }
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }
}

