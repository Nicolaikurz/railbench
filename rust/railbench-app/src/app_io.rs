use crate::*;

impl RailbenchApp {
    pub fn release_3d_cpu_memory(&mut self) {
        self.scene_3d_bundle = None;
        self.scene_3d_cache.clear();
        self.mesh_3d_key = (u64::MAX, true, 0, true, 0, 0, 0);
    }

    fn trim_undo_stack(&mut self) {
        let mut total_bytes: usize = self.undo_stack.iter().map(estimate_layout_ram_bytes).sum();
        while self.undo_stack.len() > MAX_UNDO_STEPS
            || (self.undo_stack.len() > MIN_UNDO_STEPS && total_bytes > UNDO_MEMORY_BUDGET_BYTES)
        {
            if let Some(evicted) = self.undo_stack.pop_front() {
                total_bytes = total_bytes.saturating_sub(estimate_layout_ram_bytes(&evicted));
            } else {
                break;
            }
        }
    }

    pub fn generate_stress_tracks(&mut self, count: usize) {
        let Some(entry) = self
            .catalogs
            .get(self.selected_catalog_idx)
            .and_then(|c| c.entries.iter().find(|e| e.kind == "straight").or_else(|| c.entries.first()))
            .or_else(|| self.catalogs.iter().flat_map(|c| c.entries.iter()).next())
            .cloned()
        else {
            return;
        };

        self.push_undo();
        let step_x = entry.length.unwrap_or(180.0).max(120.0) + 20.0;
        let step_y = 65.0;
        let cols = (count as f64).sqrt().ceil().max(1.0) as usize;
        let rows = (count + cols - 1) / cols;
        let ox = -((cols as f64 - 1.0) * step_x) * 0.5;
        let oy = -((rows as f64 - 1.0) * step_y) * 0.5;
        let layer_id = self.layout.active_layer_id.clone();

        for i in 0..count {
            let col = i % cols;
            let row = i / cols;
            let id = format!("stress-{}", self.next_piece_seq);
            self.next_piece_seq += 1;
            let pc = create_piece(
                id,
                &entry,
                Transform {
                    x: ox + col as f64 * step_x,
                    y: oy + row as f64 * step_y,
                    rotation: 0.0,
                },
                layer_id.clone(),
            );
            self.layout.pieces.push(pc);
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        self.mark_layout_dirty();
        self.status_message = Some(format!("Stress test: +{count} tracks added"));
    }

    pub fn mark_layout_dirty(&mut self) {
        self.layout_version = self.layout_version.wrapping_add(1);
        self.collab.send_layout(&self.layout);
    }

    pub fn from_saved_settings() -> Self {
        let s = AppSettings::load_from_disk();
        let mut app = Self::default();
        app.persist_settings = true;
        app.dark_mode = s.dark_mode;
        app.lang = if s.lang == "en" { Lang::En } else { Lang::De };
        app.snapping_enabled = s.snapping_enabled;
        app.layer_protection = s.layer_protection;
        app.color_mode = match s.color_mode.as_str() {
            "layers" => ColorMode::Layers,
            "off" => ColorMode::Off,
            _ => ColorMode::Types,
        };
        app.show_labels = s.show_labels;
        app.show_connectors = s.show_connectors;
        app.show_rulers = s.show_rulers;
        app.show_wood_plates = s.show_wood_plates;
        app.show_right_sidebar = s.show_right_sidebar;
        app.rotation_step = s.rotation_step;
        app.viewer3d_env = match s.viewer3d_env.as_str() {
            "minimal" => Viewer3DEnv::Minimal,
            _ => Viewer3DEnv::HallLandscape,
        };
        app.perf_monitor.visible = s.perf_monitor_visible;
        app.app_settings = s;
        app.inventory = crate::inventory_store::TrackInventoryStore::load_from_disk();

        match app.app_settings.startup_action {
            StartupAction::Ask => {
                app.show_welcome_dialog = true;
            }
            StartupAction::Recent => {
                if let Some(ref recent) = app.app_settings.last_opened_path.clone() {
                    if std::path::Path::new(recent).exists() {
                        app.open_layout_path_internal(recent, false);
                        app.undo_stack.clear();
                        app.redo_stack.clear();
                    }
                }
            }
            StartupAction::New => {
                app.layout = make_empty_layout();
                app.selected.clear();
                app.undo_stack.clear();
                app.redo_stack.clear();
                app.pending_fit_view = true;
            }
            StartupAction::Demo => {
                if app.layout.pieces.is_empty()
                    && std::path::Path::new("demotracks/mainline-h0.railbench").exists()
                {
                    app.open_layout_path_internal("demotracks/mainline-h0.railbench", false);
                    app.undo_stack.clear();
                    app.redo_stack.clear();
                }
            }
        }
        app
    }

    pub fn save_current_settings(&mut self) {
        self.app_settings.dark_mode = self.dark_mode;
        self.app_settings.lang = match self.lang {
            Lang::De => "de",
            Lang::En => "en",
        }
        .into();
        self.app_settings.snapping_enabled = self.snapping_enabled;
        self.app_settings.layer_protection = self.layer_protection;
        self.app_settings.color_mode = match self.color_mode {
            ColorMode::Types => "types",
            ColorMode::Layers => "layers",
            ColorMode::Off => "off",
        }
        .into();
        self.app_settings.show_labels = self.show_labels;
        self.app_settings.show_connectors = self.show_connectors;
        self.app_settings.show_rulers = self.show_rulers;
        self.app_settings.show_wood_plates = self.show_wood_plates;
        self.app_settings.show_right_sidebar = self.show_right_sidebar;
        self.app_settings.rotation_step = self.rotation_step;
        self.app_settings.viewer3d_env = match self.viewer3d_env {
            Viewer3DEnv::HallLandscape => "hall-landscape",
            Viewer3DEnv::Minimal => "minimal",
        }
        .into();
        self.app_settings.perf_monitor_visible = self.perf_monitor.visible;
        if self.persist_settings {
            self.app_settings.save_to_disk();
        }
    }

    pub fn reset_personal_settings(&mut self) {
        self.app_settings.reset_personal_defaults();
        self.dark_mode = self.app_settings.dark_mode;
        self.lang = if self.app_settings.lang == "en" {
            Lang::En
        } else {
            Lang::De
        };
        self.snapping_enabled = self.app_settings.snapping_enabled;
        self.layer_protection = self.app_settings.layer_protection;
        self.color_mode = ColorMode::Types;
        self.show_labels = self.app_settings.show_labels;
        self.show_connectors = self.app_settings.show_connectors;
        self.show_rulers = self.app_settings.show_rulers;
        self.show_wood_plates = self.app_settings.show_wood_plates;
        self.show_right_sidebar = self.app_settings.show_right_sidebar;
        self.rotation_step = self.app_settings.rotation_step;
        self.viewer3d_env = Viewer3DEnv::HallLandscape;
        self.perf_monitor.visible = self.app_settings.perf_monitor_visible;
        if self.persist_settings {
            self.app_settings.save_to_disk();
        }
    }

    pub fn bulk_equip_turnouts(&mut self, equip: bool) -> usize {
        self.push_undo();
        let mut count = 0usize;
        for p in &mut self.layout.pieces {
            let is_sw = crate::catalog::find_entry(&self.catalogs, &p.catalog_id)
                .map_or(false, |e| is_turnout_kind(&e.kind));
            if is_sw {
                p.has_lantern = Some(equip);
                p.has_decoder = Some(equip);
                count += 1;
            }
        }
        self.mark_layout_dirty();
        self.status_message = Some(if equip {
            format!("Equipped {count} turnouts with lanterns & decoders")
        } else {
            format!("Cleared lanterns & decoders on {count} turnouts")
        });
        count
    }

    pub fn export_bom_csv_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name(format!("{}-bom.csv", self.layout.name))
            .save_file()
        {
            let csv = export_bom_csv(&self.layout, &self.catalogs, self.lang == Lang::De);
            match std::fs::write(&path, csv) {
                Ok(_) => self.status_message = Some(format!("Exported CSV: {}", path.display())),
                Err(e) => self.status_message = Some(format!("CSV export error: {e}")),
            }
        }
    }

    pub fn export_xml_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RAILbench XML", &["xml"])
            .set_file_name(format!("{}.xml", self.layout.name))
            .save_file()
        {
            let xml = export_layout_xml(&self.layout);
            match std::fs::write(&path, xml) {
                Ok(_) => self.status_message = Some(format!("Exported XML: {}", path.display())),
                Err(e) => self.status_message = Some(format!("XML export error: {e}")),
            }
        }
    }

    pub fn export_svg_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SVG Blueprint", &["svg"])
            .set_file_name(format!("{}.svg", self.layout.name))
            .save_file()
        {
            let svg = export_layout_svg_with_options(&self.layout, &self.catalogs, &self.export_opts);
            match std::fs::write(&path, svg) {
                Ok(_) => self.status_message = Some(format!("Exported SVG: {}", path.display())),
                Err(e) => self.status_message = Some(format!("SVG export error: {e}")),
            }
        }
    }

    pub fn export_pdf_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF Blueprint", &["pdf"])
            .set_file_name(format!("{}.pdf", self.layout.name))
            .save_file()
        {
            let pdf = export_layout_pdf_with_options(
                &self.layout,
                &self.catalogs,
                &self.export_opts,
                self.lang == Lang::De,
            );
            match std::fs::write(&path, pdf) {
                Ok(_) => self.status_message = Some(format!("Exported PDF: {}", path.display())),
                Err(e) => self.status_message = Some(format!("PDF export error: {e}")),
            }
        }
    }

    pub fn toggle_3d(&mut self) {
        self.is_3d_mode = !self.is_3d_mode;
        if self.is_3d_mode {
            self.camera_3d.fit_layout(&self.layout);
            self.ensure_mesh_3d();
        } else {
            self.release_3d_cpu_memory();
        }
    }

    pub fn current_short_circuit_points(&self) -> Vec<Vec2> {
        if let Some((ver, ref sim)) = self.power_sim_cache {
            if ver == self.layout_version {
                return sim
                    .short_circuit_points
                    .iter()
                    .map(|p| Vec2 { x: p.x, y: p.y })
                    .collect();
            }
        }
        Vec::new()
    }

    pub fn ensure_mesh_3d(&mut self) {
        if self.power_sim_enabled {
            let _ = self.get_or_update_power_sim();
        }
        let sc_pts = self.current_short_circuit_points();
        let cm_u8 = match self.color_mode {
            ColorMode::Off => 0,
            ColorMode::Types => 1,
            ColorMode::Layers => 2,
        };
        let key = (
            self.layout_version,
            self.show_wood_plates,
            self.viewer3d_env as u8,
            self.dark_mode,
            cm_u8,
            sc_pts.len(),
            self.vehicle_sim
                .turnout_version
                .wrapping_add(self.vehicle_sim.bridge_mesh_version.wrapping_mul(31))
                .wrapping_add(self.train_3d.turnout_branches_version.wrapping_mul(131)),
        );
        if self.mesh_3d_key != key || self.scene_3d_bundle.is_none() {
            let opts = Scene3DOptions {
                show_wood_plates: self.show_wood_plates,
                env_mode: self.viewer3d_env,
                dark_mode: self.dark_mode,
                color_mode: self.color_mode,
                time_of_day: self.time_of_day,
                short_circuit_points: sc_pts,
                turnout_states: self.vehicle_sim.turnout_states.clone(),
                turntable_angles_deg: self.vehicle_sim.turntable_angles_deg.clone(),
                transfer_offsets_mm: self.vehicle_sim.transfer_offsets_mm.clone(),
            };
            self.scene_3d_bundle = Some(viewer3d::update_scene_3d_cache(
                &self.layout,
                &self.catalogs,
                &opts,
                &mut self.scene_3d_cache,
            ));
            self.mesh_3d_key = key;
        }
    }

    pub fn get_or_update_power_sim(&mut self) -> &PowerSimResult {
        let ver = self.layout_version;
        let stale = self
            .power_sim_cache
            .as_ref()
            .map_or(true, |(cached_ver, _)| *cached_ver != ver);
        if stale {
            let res = simulate_layout_power(&self.layout, &self.catalogs);
            self.power_sim_cache = Some((ver, Arc::new(res)));
        }
        self.power_sim_cache.as_ref().unwrap().1.as_ref()
    }

    pub fn focus_world_point(&mut self, wx: f64, wy: f64) {
        self.pending_focus_point = Some(Vec2 { x: wx, y: wy });
        if self.is_3d_mode {
            self.is_3d_mode = false;
            self.release_3d_cpu_memory();
        }
    }

    pub fn push_undo(&mut self) {
        self.undo_stack.push_back(self.layout.clone());
        self.trim_undo_stack();
        self.redo_stack.clear();
        self.mark_layout_dirty();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(self.layout.clone());
            while self.redo_stack.len() > MAX_UNDO_STEPS {
                self.redo_stack.pop_front();
            }
            self.layout = prev;
            self.selected.clear();
            self.mark_layout_dirty();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(self.layout.clone());
            self.trim_undo_stack();
            self.layout = next;
            self.selected.clear();
            self.mark_layout_dirty();
        }
    }

    pub fn new_empty_layout(&mut self) {
        self.push_undo();
        self.canvas_2d_cache.clear_proto_caches();
        if !self.is_3d_mode {
            self.release_3d_cpu_memory();
        }
        self.layout = make_empty_layout();
        self.selected.clear();
        self.pending_fit_view = true;
        self.status_message = Some("New layout created".into());
    }

    pub fn open_layout_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RAILbench Layout", &["railbench", "gleisplan", "json", "xml"])
            .pick_file()
        {
            if let Some(s) = path.to_str() {
                self.open_layout_path(s);
            }
        }
    }

    pub fn is_favorite_track(&self, id: &str) -> bool {
        self.app_settings.favorite_tracks.iter().any(|f| f == id)
    }

    pub fn toggle_favorite_track(&mut self, id: &str) {
        if let Some(pos) = self.app_settings.favorite_tracks.iter().position(|f| f == id) {
            self.app_settings.favorite_tracks.remove(pos);
        } else {
            self.app_settings.favorite_tracks.push(id.to_string());
        }
        self.save_current_settings();
    }

    pub fn import_catalog_json_str(&mut self, raw: &str) -> Result<String, String> {
        let res = crate::catalog_store::import_and_persist_catalog(&mut self.catalogs, raw)?;
        if res.is_road {
            self.selected_road_catalog_idx = res.index;
        } else {
            self.selected_catalog_idx = res.index;
        }
        self.mark_layout_dirty();
        Ok(res.name)
    }

    pub fn import_catalog_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Track Catalog JSON", &["json"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(raw) => match self.import_catalog_json_str(&raw) {
                    Ok(name) => {
                        self.status_message = Some(format!("Imported catalog '{name}'"));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Catalog import error: {e}"));
                    }
                },
                Err(e) => {
                    self.status_message = Some(format!("Read error: {e}"));
                }
            }
        }
    }

    pub fn open_layout_path(&mut self, path: &str) {
        self.open_layout_path_internal(path, true);
    }

    pub(crate) fn open_layout_path_internal(&mut self, path: &str, record_recent: bool) {
        match std::fs::read_to_string(path) {
            Ok(raw) => {
                let trimmed = raw.trim_start();
                let parsed = if path.to_ascii_lowercase().ends_with(".xml") || trimmed.starts_with('<') {
                    import_layout_xml(&raw, &self.catalogs)
                } else {
                    load_layout_json(&raw).map_err(|e| e.to_string())
                };
                match parsed {
                    Ok(mut loaded) => {
                    if loaded.layers.as_ref().map_or(true, |l| l.is_empty()) {
                        loaded.layers = Some(vec![default_layer()]);
                        loaded.active_layer_id = Some("layer-1".into());
                    }
                    let first_board_id = board_tools::ensure_baseboards(&mut loaded)
                        .first()
                        .and_then(|b| b.id.clone());
                    self.active_board_id = first_board_id;
                    self.dragging_board = None;
                    self.dragging_model_rot = None;
                    self.board_snap_guides.clear();
                    self.sketcher.stop();
                    self.model_scale = model_tools::parse_scale_code(&loaded.scale).into();
                    if let Some(models) = &mut loaded.models {
                        dedup_model_file_data(models, &self.custom_models_library);
                        for m in models.iter() {
                            if m.file_data.is_some()
                                && !self.custom_models_library.iter().any(|c| c.name == m.name)
                            {
                                self.custom_models_library.push(m.clone());
                            }
                        }
                    }
                    self.canvas_2d_cache.clear_proto_caches();
                    if !self.is_3d_mode {
                        self.release_3d_cpu_memory();
                    }
                    update_layout_connections(&mut loaded, 4.0, 35.0, 1.0);
                    self.push_undo();
                    self.layout = loaded;
                    self.selected.clear();
                    self.pending_fit_view = true;
                    self.status_message = Some(format!("Loaded {path}"));
                    if record_recent {
                        self.app_settings.last_opened_path = Some(path.to_string());
                        self.save_current_settings();
                    }
                }
                Err(e) => self.status_message = Some(format!("Parse error: {e}")),
                }
            }
            Err(e) => self.status_message = Some(format!("Read error: {e}")),
        }
    }

    pub fn save_layout_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RAILbench Layout", &["railbench", "json"])
            .set_file_name(format!("{}.railbench", self.layout.name))
            .save_file()
        {
            match save_layout_json(&self.layout) {
                Ok(json) => match std::fs::write(&path, json) {
                    Ok(_) => {
                        self.status_message = Some(format!("Saved {}", path.display()));
                        if let Some(s) = path.to_str() {
                            self.app_settings.last_opened_path = Some(s.to_string());
                            self.save_current_settings();
                        }
                    }
                    Err(e) => self.status_message = Some(format!("Write error: {e}")),
                },
                Err(e) => self.status_message = Some(format!("Serialize error: {e}")),
            }
        }
    }

}
