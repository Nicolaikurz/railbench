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

    pub fn show_toast(&mut self, msg: impl Into<String>) {
        let s: String = msg.into();
        self.status_message = Some(s.clone());
        self.toast_message = Some((s, std::time::Instant::now()));
    }

    pub fn is_dirty(&self) -> bool {
        self.layout_version != self.saved_layout_version
    }

    pub fn autosave_path() -> Option<std::path::PathBuf> {
        AppSettings::config_dir().map(|d| d.join("autosave.railbench"))
    }

    pub fn clear_autosave(&mut self) {
        if let Some(path) = Self::autosave_path() {
            let _ = std::fs::remove_file(path);
        }
        self.last_autosave_version = self.layout_version;
    }

    pub fn tick_autosave(&mut self) {
        if !self.persist_settings || self.layout_version == self.last_autosave_version {
            return;
        }
        if !self.is_dirty() {
            self.last_autosave_version = self.layout_version;
            return;
        }
        let Some(edited_at) = self.last_edit_instant else {
            return;
        };
        if edited_at.elapsed() < std::time::Duration::from_secs(3) {
            return;
        }
        if let Some(path) = Self::autosave_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = save_layout_json(&self.layout) {
                if std::fs::write(path, json).is_ok() {
                    self.last_autosave_version = self.layout_version;
                }
            }
        }
    }

    pub fn restore_autosave(&mut self) {
        if let Some(path) = Self::autosave_path() {
            if let Ok(raw) = std::fs::read_to_string(&path) {
                self.open_layout_json_str(&raw, "autosave", None);
                self.show_toast(if self.lang == Lang::De {
                    "Ungesicherte Sitzung wiederhergestellt"
                } else {
                    "Restored unsaved session"
                });
            }
        }
        self.show_autosave_recovery = false;
    }

    pub fn discard_autosave(&mut self) {
        self.clear_autosave();
        self.show_autosave_recovery = false;
    }

    pub fn mark_layout_dirty(&mut self) {
        self.layout_version = self.layout_version.wrapping_add(1);
        self.last_edit_instant = Some(std::time::Instant::now());
        self.collab.send_layout(&self.layout);
    }

    pub fn from_saved_settings() -> Self {
        let s = AppSettings::load_from_disk();
        let mut app = Self::default();
        app.persist_settings = true;
        app.dark_mode = s.dark_mode;
        app.lang = if s.lang == "auto" {
            if crate::settings::detect_os_lang_de() { Lang::De } else { Lang::En }
        } else if s.lang == "en" {
            Lang::En
        } else {
            Lang::De
        };
        if app.lang == Lang::En {
            app.layout = make_empty_layout_for_lang(false);
        }
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
        app.rotation_snap_locked = s.rotation_snap_locked;
        app.power_sim_enabled = s.power_sim_enabled;
        app.bars_locked = s.bars_locked;
        app.low_gpu_mode = s.low_gpu_mode;
        app.use_discrete_gpu = s.use_discrete_gpu;
        app.sidebar_tab = match s.active_sidebar_tab.as_str() {
            "roads" => SidebarTab::Roads,
            "models" => SidebarTab::Models,
            "terrain" => SidebarTab::Terrain,
            "board" => SidebarTab::Board,
            "bom" => SidebarTab::Bom,
            _ => SidebarTab::Tracks,
        };
        if let Some(ref cid) = s.selected_catalog_id {
            if let Some(idx) = app.catalogs.iter().position(|c| &c.id == cid) {
                if app.catalogs[idx].is_road == Some(true) {
                    app.selected_road_catalog_idx = idx;
                } else {
                    app.selected_catalog_idx = idx;
                }
            }
        }
        if let Some(off) = s.camera_offset {
            app.camera.offset = egui::vec2(off[0], off[1]);
        }
        if let Some(z) = s.camera_zoom {
            app.camera.zoom = z;
        }
        if let Some(ref n) = s.collab_user_name {
            if !n.trim().is_empty() {
                app.collab.user_name = n.clone();
            }
        }
        if let Some(ref c) = s.collab_user_color {
            if !c.trim().is_empty() {
                app.collab.user_color = c.clone();
            }
        }
        if let Some(ref pw) = s.collab_password {
            app.collab.password = pw.clone();
        }
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
                app.layout = make_empty_layout_for_lang(app.lang == Lang::De);
                app.current_file_path = None;
                app.saved_layout_version = app.layout_version;
                app.selected.clear();
                app.undo_stack.clear();
                app.redo_stack.clear();
                app.pending_fit_view = true;
            }
            StartupAction::Demo => {
                if app.layout.pieces.is_empty() {
                    app.open_demo_layout("demotracks/mainline-h0.railbench", false);
                    app.undo_stack.clear();
                    app.redo_stack.clear();
                }
            }
            StartupAction::Open => {
                app.pending_open_on_start = true;
            }
        }
        app.saved_layout_version = app.layout_version;
        app.last_autosave_version = app.layout_version;
        if let Some(auto_p) = Self::autosave_path() {
            if auto_p.exists() {
                if let Ok(raw) = std::fs::read_to_string(&auto_p) {
                    if load_layout_json(&raw).map_or(false, |l| !l.pieces.is_empty()) {
                        app.show_autosave_recovery = true;
                    }
                }
            }
        }
        app
    }

    pub fn save_current_settings(&mut self) {
        self.app_settings.dark_mode = self.dark_mode;
        if self.app_settings.lang != "auto" {
            self.app_settings.lang = match self.lang {
                Lang::De => "de",
                Lang::En => "en",
            }
            .into();
        }
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
        self.app_settings.rotation_snap_locked = self.rotation_snap_locked;
        self.app_settings.power_sim_enabled = self.power_sim_enabled;
        self.app_settings.bars_locked = self.bars_locked;
        self.app_settings.low_gpu_mode = self.low_gpu_mode;
        self.app_settings.use_discrete_gpu = self.use_discrete_gpu;
        self.app_settings.selected_catalog_id = self
            .catalogs
            .get(self.selected_catalog_idx)
            .map(|c| c.id.clone());
        self.app_settings.active_sidebar_tab = match self.sidebar_tab {
            SidebarTab::Tracks => "tracks",
            SidebarTab::Roads => "roads",
            SidebarTab::Models => "models",
            SidebarTab::Terrain => "terrain",
            SidebarTab::Board => "board",
            SidebarTab::Bom => "bom",
        }
        .into();
        self.app_settings.camera_offset = Some([self.camera.offset.x, self.camera.offset.y]);
        self.app_settings.camera_zoom = Some(self.camera.zoom);
        if !self.collab.user_name.trim().is_empty() {
            self.app_settings.collab_user_name = Some(self.collab.user_name.clone());
        }
        if !self.collab.user_color.trim().is_empty() {
            self.app_settings.collab_user_color = Some(self.collab.user_color.clone());
        }
        self.app_settings.collab_password = if self.collab.password.is_empty() {
            None
        } else {
            Some(self.collab.password.clone())
        };
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
        self.bars_locked = self.app_settings.bars_locked;
        self.low_gpu_mode = self.app_settings.low_gpu_mode;
        self.use_discrete_gpu = self.app_settings.use_discrete_gpu;
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

    /// Generates the blueprint PDF into a temporary file and opens it in the OS viewer/printer dialog.
    pub fn print_or_open_pdf(&mut self) {
        let pdf = export_layout_pdf_with_options(
            &self.layout,
            &self.catalogs,
            &self.export_opts,
            self.lang == Lang::De,
        );
        let safe_name: String = self
            .layout
            .name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let stem = if safe_name.is_empty() { "layout" } else { safe_name.as_str() };
        let path = std::env::temp_dir().join(format!("railbench-print-{stem}.pdf"));
        if let Err(e) = std::fs::write(&path, pdf) {
            self.status_message = Some(format!("PDF write error: {e}"));
            return;
        }
        let path_str = path.to_string_lossy().into_owned();
        #[cfg(target_os = "linux")]
        let res = std::process::Command::new("xdg-open").arg(&path_str).spawn();
        #[cfg(target_os = "macos")]
        let res = std::process::Command::new("open").arg(&path_str).spawn();
        #[cfg(target_os = "windows")]
        let res = std::process::Command::new("cmd")
            .args(["/C", "start", "", &path_str])
            .spawn();

        match res {
            Ok(_) => {
                self.show_toast(if self.lang == Lang::De {
                    "PDF zum Drucken geoeffnet"
                } else {
                    "Opened PDF for printing"
                });
            }
            Err(e) => {
                self.status_message = Some(format!("Could not open PDF viewer: {e}"));
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

    pub fn current_short_circuit_points(&self) -> Vec<[f64; 3]> {
        if let Some((ver, ref sim)) = self.power_sim_cache {
            if ver == self.layout_version {
                return sim
                    .short_circuit_points
                    .iter()
                    .map(|pt| {
                        let elev = self
                            .layout
                            .pieces
                            .iter()
                            .find(|pc| pc.id == pt.piece_id_1)
                            .map(|pc| {
                                let lh = self
                                    .layout
                                    .layers
                                    .as_ref()
                                    .and_then(|ls| {
                                        ls.iter().find(|l| {
                                            l.id == pc.layer_id.as_deref().unwrap_or("layer-1")
                                        })
                                    })
                                    .and_then(|l| l.height_mm)
                                    .unwrap_or(0.0);
                                let pe = pc
                                    .elevation
                                    .map(|e| (e.start + e.end) * 0.5)
                                    .unwrap_or(0.0);
                                lh + pe
                            })
                            .unwrap_or(0.0);
                        [pt.x, elev, pt.y]
                    })
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
                .wrapping_add(self.vehicle_sim.bridge_mesh_version.wrapping_mul(31)),
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
                active_uncouplers: self.vehicle_sim.active_uncoupler_ids(),
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
            self.vehicle_sim.sanitize_for_layout(&self.layout, &self.catalogs);
            self.mark_layout_dirty();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(self.layout.clone());
            self.trim_undo_stack();
            self.layout = next;
            self.selected.clear();
            self.vehicle_sim.sanitize_for_layout(&self.layout, &self.catalogs);
            self.mark_layout_dirty();
        }
    }

    pub fn new_empty_layout(&mut self) {
        self.push_undo();
        self.canvas_2d_cache.clear_proto_caches();
        if !self.is_3d_mode {
            self.release_3d_cpu_memory();
        }
        self.layout = make_empty_layout_for_lang(self.lang == Lang::De);
        self.vehicle_sim.clear_all();
        self.current_file_path = None;
        self.selected.clear();
        self.pending_fit_view = true;
        self.saved_layout_version = self.layout_version;
        self.clear_autosave();
        self.show_toast(if self.lang == Lang::De {
            "Neue leere Anlage erstellt"
        } else {
            "New layout created"
        });
    }

    pub fn request_new_layout(&mut self) {
        if self.is_dirty() {
            self.pending_confirm_action = Some(PendingConfirmAction::New);
        } else {
            self.new_empty_layout();
        }
    }

    pub fn request_open_layout_dialog(&mut self) {
        if self.is_dirty() {
            self.pending_confirm_action = Some(PendingConfirmAction::Open);
        } else {
            self.open_layout_dialog();
        }
    }

    pub fn request_open_demo(&mut self, path: &str) {
        if self.is_dirty() {
            self.pending_confirm_action = Some(PendingConfirmAction::Demo(path.to_string()));
        } else {
            self.open_demo_layout(path, true);
        }
    }

    pub fn embedded_demo_json(path: &str) -> Option<&'static str> {
        let norm = path.replace('\\', "/");
        match norm.as_str() {
            "demotracks/alpenland-h0.railbench" => Some(include_str!("../../../demotracks/alpenland-h0.railbench")),
            "demotracks/mainline-h0.railbench" => Some(include_str!("../../../demotracks/mainline-h0.railbench")),
            "demotracks/shunting-0.railbench" => Some(include_str!("../../../demotracks/shunting-0.railbench")),
            "demotracks/switching-n.railbench" => Some(include_str!("../../../demotracks/switching-n.railbench")),
            "demotracks/garden-g.railbench" => Some(include_str!("../../../demotracks/garden-g.railbench")),
            "demotracks/suitcase-z.railbench" => Some(include_str!("../../../demotracks/suitcase-z.railbench")),
            _ => None,
        }
    }

    pub fn demo_preview_meta(&self, path: &str) -> Option<String> {
        let raw = if std::path::Path::new(path).exists() {
            std::fs::read_to_string(path).ok()?
        } else {
            Self::embedded_demo_json(path)?.to_string()
        };
        let layout = load_layout_json(&raw).ok()?;
        let bb = layout
            .baseboards
            .as_ref()
            .and_then(|bs| bs.first())
            .or(layout.baseboard.as_ref());
        let dims = bb
            .map(|b| format!("{:.0}x{:.0} cm", b.width / 10.0, b.depth / 10.0))
            .unwrap_or_default();
        let pieces = layout.pieces.len();
        Some(if self.lang == Lang::De {
            format!("Spur {}  ·  {}  ·  {} Gleise", layout.scale, dims, pieces)
        } else {
            format!("Scale {}  ·  {}  ·  {} tracks", layout.scale, dims, pieces)
        })
    }

    pub fn open_demo_layout(&mut self, path: &str, record_recent: bool) {
        if std::path::Path::new(path).exists() {
            self.open_layout_path_internal(path, record_recent);
            return;
        }
        if let Some(raw) = Self::embedded_demo_json(path) {
            self.open_layout_json_str(raw, path, None);
            self.current_file_path = None;
            self.saved_layout_version = self.layout_version;
            self.clear_autosave();
        } else {
            self.status_message = Some(format!("Demo not found: {path}"));
        }
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

    pub fn is_favorite_track_entry(&self, catalog_id: &str, track_id: &str) -> bool {
        let scoped = format!("{catalog_id}::{track_id}");
        self.app_settings
            .favorite_tracks
            .iter()
            .any(|f| f == &scoped || f == track_id)
    }

    pub fn toggle_favorite_track_entry(&mut self, catalog_id: &str, track_id: &str) {
        let scoped = format!("{catalog_id}::{track_id}");
        if let Some(pos) = self
            .app_settings
            .favorite_tracks
            .iter()
            .position(|f| f == &scoped || f == track_id)
        {
            self.app_settings.favorite_tracks.remove(pos);
        } else {
            self.app_settings.favorite_tracks.push(scoped);
        }
        self.save_current_settings();
    }

    pub fn export_catalog_dialog(&mut self, cat_idx: usize) {
        let Some(cat) = self.catalogs.get(cat_idx) else {
            return;
        };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Track Catalog JSON", &["json"])
            .set_file_name(format!("{}.json", cat.id))
            .save_file()
        {
            let json = crate::catalog_store::export_catalog_json(cat);
            match std::fs::write(&path, json) {
                Ok(_) => self.show_toast(if self.lang == Lang::De {
                    format!("Katalog exportiert: {}", cat.name)
                } else {
                    format!("Exported catalog: {}", cat.name)
                }),
                Err(e) => self.status_message = Some(format!("Catalog export error: {e}")),
            }
        }
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
                let rec = if record_recent { Some(path) } else { None };
                self.open_layout_json_str(&raw, path, rec);
            }
            Err(e) => self.status_message = Some(format!("Read error: {e}")),
        }
    }

    pub(crate) fn open_layout_json_str(&mut self, raw: &str, source_label: &str, record_path: Option<&str>) {
        let trimmed = raw.trim_start();
        let parsed = if source_label.to_ascii_lowercase().ends_with(".xml") || trimmed.starts_with('<') {
            import_layout_xml(raw, &self.catalogs)
        } else {
            load_layout_json(raw).map_err(|e| e.to_string())
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
                self.dragging_track_rot = None;
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
                crate::catalog::disconnect_cross_type_connections(&mut loaded, &self.catalogs);
                self.push_undo();
                self.layout = loaded;
                self.vehicle_sim.clear_all();
                self.selected.clear();
                self.pending_fit_view = true;
                self.current_file_path = if std::path::Path::new(source_label).exists() {
                    Some(source_label.to_string())
                } else {
                    None
                };
                self.saved_layout_version = self.layout_version;
                self.clear_autosave();
                self.status_message = Some(format!("Loaded {source_label}"));
                if let Some(rp) = record_path {
                    self.app_settings.push_recent_file(rp);
                    self.save_current_settings();
                }
            }
            Err(e) => self.status_message = Some(format!("Parse error: {e}")),
        }
    }

    fn write_layout_to_path(&mut self, path: &std::path::Path) -> bool {
        match save_layout_json(&self.layout) {
            Ok(json) => match std::fs::write(path, json) {
                Ok(_) => {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if self.layout.name == "Neue Anlage"
                            || self.layout.name == "New Layout"
                            || self.layout.name.trim().is_empty()
                        {
                            self.layout.name = stem.to_string();
                        }
                    }
                    if let Some(s) = path.to_str() {
                        self.current_file_path = Some(s.to_string());
                        self.app_settings.push_recent_file(s);
                        self.save_current_settings();
                    }
                    self.saved_layout_version = self.layout_version;
                    self.clear_autosave();
                    self.show_toast(if self.lang == Lang::De {
                        format!("Gespeichert: {}", path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
                    } else {
                        format!("Saved {}", path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
                    });
                    true
                }
                Err(e) => {
                    self.status_message = Some(format!("Write error: {e}"));
                    false
                }
            },
            Err(e) => {
                self.status_message = Some(format!("Serialize error: {e}"));
                false
            }
        }
    }

    pub fn save_layout_dialog(&mut self) -> bool {
        if let Some(ref existing) = self.current_file_path.clone() {
            let p = std::path::PathBuf::from(existing);
            return self.write_layout_to_path(&p);
        }
        self.save_layout_as_dialog()
    }

    pub fn save_layout_as_dialog(&mut self) -> bool {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RAILbench Layout", &["railbench", "json"])
            .set_file_name(format!("{}.railbench", self.layout.name))
            .save_file()
        {
            return self.write_layout_to_path(&path);
        }
        false
    }

}
