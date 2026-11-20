mod board_tools;
mod building_snap;
mod canvas2d;
mod catalog;
mod collab;
mod export;
mod i18n;
mod icons;
mod model_tools;
mod models3d_mesh;
mod perf_monitor;
mod power_sim;
mod ramp_tools;
mod settings;
mod terrain_tools;
mod theme;
mod ui_panels;
mod viewer3d;

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use eframe::egui;
use railbench_core::geometry::{norm_angle_deg, rotate};
use railbench_core::io::{load_layout_json, save_layout_json};
use railbench_core::snapping::update_layout_connections;
use railbench_core::types::{
    BaseboardConfig, Catalog, CatalogEntry, Elevation, Layer, Layout, Model3D, TrackPiece, Transform, Vec2,
};

use crate::canvas2d::{draw_canvas_2d, Camera2D, ColorMode};
use crate::catalog::{builtin_catalogs, create_piece};
use crate::collab::CollabClient;
use crate::export::{
    export_bom_csv, export_layout_pdf_with_options, export_layout_svg_with_options,
    export_layout_xml, BlueprintExportOptions,
};
use crate::i18n::Lang;
use crate::power_sim::{simulate_layout_power, PowerSimResult};
use crate::ramp_tools::{RampProfile, RampShape};
use crate::settings::{AppSettings, KeyAction, KeyCategory, StartupAction};
use crate::ui_panels::{
    default_layer, draw_dialogs, draw_floating_selection_bar, draw_left_sidebar, draw_right_sidebar, draw_top_bars,
};
use crate::viewer3d::{
    build_scene_mesh_3d, draw_viewport_3d, init_wgpu_3d_resources, Camera3D, Scene3DOptions,
    SceneMesh3D, Viewer3DEnv,
};

#[derive(Debug, Clone)]
pub struct ClipboardData {
    pub pieces: Vec<TrackPiece>,
    pub models: Vec<Model3D>,
}

pub fn is_turnout_kind(kind: &str) -> bool {
    matches!(
        kind,
        "turnout-left"
            | "turnout-right"
            | "three-way"
            | "double-slip"
            | "curved-turnout-left"
            | "curved-turnout-right"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Tracks,
    Roads,
    Models,
    Terrain,
    Board,
    Bom,
}

pub struct RailbenchApp {
    pub layout: Layout,
    pub catalogs: Vec<Catalog>,
    pub selected_catalog_idx: usize,
    pub selected_road_catalog_idx: usize,
    pub catalog_search: String,
    pub sidebar_tab: SidebarTab,
    pub camera: Camera2D,
    pub camera_3d: Camera3D,
    pub is_3d_mode: bool,
    pub mesh_3d: Option<Arc<SceneMesh3D>>,
    pub mesh_3d_version: u64,
    pub layout_version: u64,
    pub selected: HashSet<String>,
    pub undo_stack: Vec<Layout>,
    pub redo_stack: Vec<Layout>,
    pub clipboard: Option<ClipboardData>,
    pub paste_offset_count: usize,
    pub last_placed_catalog_id: Option<String>,
    pub dragging_active: bool,
    pub drag_last_world: Option<Vec2>,
    pub dragging_flex_handle: Option<(String, usize)>,
    pub dragging_catalog_entry: Option<CatalogEntry>,
    pub dark_mode: bool,
    pub lang: Lang,
    pub color_mode: ColorMode,
    pub show_labels: bool,
    pub show_connectors: bool,
    pub show_rulers: bool,
    pub snapping_enabled: bool,
    pub power_sim_enabled: bool,
    pub power_sim_cache: Option<(u64, PowerSimResult)>,
    pub pending_focus_point: Option<Vec2>,
    pub measure_mode: bool,
    pub marquee_start: Option<egui::Pos2>,
    pub measure_points: Option<(Vec2, Vec2)>,
    pub rotation_step: f64,
    pub show_power_dialog: bool,
    pub show_height_dialog: bool,
    pub show_collab_dialog: bool,
    pub show_help_dialog: bool,
    pub show_right_sidebar: bool,
    pub collab: CollabClient,
    pub batch_elevation_mm: f64,
    pub height_tab: u8,
    pub ramp_start_h: f64,
    pub ramp_end_h: f64,
    pub ramp_profile: RampProfile,
    pub gen_ramp_shape: RampShape,
    pub gen_curve_left: bool,
    pub gen_target_h: f64,
    pub gen_max_grade: f64,
    pub gen_helix_turns: f64,
    pub gen_curve_id: Option<String>,
    pub terrain_brush: terrain_tools::TerrainBrushState,
    pub active_terrain_id: Option<String>,
    pub pending_fit_view: bool,
    pub status_message: Option<String>,
    pub next_piece_seq: u64,
    pub active_board_id: Option<String>,
    pub dragging_board: Option<(String, railbench_core::types::Vec2, f64, f64)>,
    pub board_snap_guides: Vec<board_tools::BaseboardSnapGuide>,
    pub show_wood_plates: bool,
    pub sketcher: board_tools::BoardSketcherState,
    pub model_scale: String,
    pub models_search: String,
    pub models_category: String,
    pub custom_models_library: Vec<Model3D>,
    pub dragging_model_rot: Option<String>,
    pub viewer3d_env: Viewer3DEnv,
    pub day_night_active: bool,
    pub time_of_day: f32,
    pub day_night_speed: f32,
    pub last_3d_frame_time: Option<std::time::Instant>,
    pub mesh_3d_key: (u64, bool, u8, bool, usize),
    pub perf_monitor: perf_monitor::PerfMonitor,
    pub app_settings: AppSettings,
    pub persist_settings: bool,
    pub bom_category: String,
    pub show_welcome_dialog: bool,
    pub show_demo_dialog: bool,
    pub show_settings_dialog: bool,
    pub show_export_dialog: bool,
    pub settings_tab: u8,
    pub help_tab: u8,
    pub keymap_search: String,
    pub keymap_category: Option<KeyCategory>,
    pub recording_key_action: Option<KeyAction>,
    pub export_opts: BlueprintExportOptions,
    pub welcome_remember_choice: bool,
}

impl Default for RailbenchApp {
    fn default() -> Self {
        let catalogs = builtin_catalogs();
        let first_road = catalogs
            .iter()
            .position(|c| c.is_road == Some(true))
            .unwrap_or(0);
        let mut app = Self {
            layout: make_empty_layout(),
            catalogs,
            selected_catalog_idx: 0,
            selected_road_catalog_idx: first_road,
            catalog_search: String::new(),
            sidebar_tab: SidebarTab::Tracks,
            camera: Camera2D::default(),
            camera_3d: Camera3D::default(),
            is_3d_mode: false,
            mesh_3d: None,
            mesh_3d_version: 0,
            layout_version: 1,
            selected: HashSet::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard: None,
            paste_offset_count: 1,
            last_placed_catalog_id: None,
            dragging_active: false,
            drag_last_world: None,
            dragging_flex_handle: None,
            dragging_catalog_entry: None,
            dark_mode: true,
            lang: Lang::De,
            color_mode: ColorMode::Types,
            show_labels: true,
            show_connectors: true,
            show_rulers: true,
            snapping_enabled: true,
            power_sim_enabled: false,
            power_sim_cache: None,
            pending_focus_point: None,
            measure_mode: false,
            marquee_start: None,
            measure_points: None,
            rotation_step: 30.0,
            show_power_dialog: false,
            show_height_dialog: false,
            show_collab_dialog: false,
            show_help_dialog: false,
            show_right_sidebar: true,
            collab: CollabClient::default(),
            batch_elevation_mm: 0.0,
            height_tab: 0,
            ramp_start_h: 0.0,
            ramp_end_h: 80.0,
            ramp_profile: RampProfile::Linear,
            gen_ramp_shape: RampShape::Straight,
            gen_curve_left: false,
            gen_target_h: 80.0,
            gen_max_grade: 2.8,
            gen_helix_turns: 1.5,
            gen_curve_id: None,
            terrain_brush: terrain_tools::TerrainBrushState::default(),
            active_terrain_id: None,
            pending_fit_view: true,
            status_message: None,
            next_piece_seq: 1,
            active_board_id: Some("board-1".into()),
            dragging_board: None,
            board_snap_guides: Vec::new(),
            show_wood_plates: true,
            sketcher: board_tools::BoardSketcherState::default(),
            model_scale: "H0".into(),
            models_search: String::new(),
            models_category: "all".into(),
            custom_models_library: Vec::new(),
            dragging_model_rot: None,
            viewer3d_env: Viewer3DEnv::Hall,
            day_night_active: false,
            time_of_day: 14.0,
            day_night_speed: 1.0,
            last_3d_frame_time: None,
            mesh_3d_key: (u64::MAX, true, 0, true, 0),
            perf_monitor: perf_monitor::PerfMonitor::default(),
            app_settings: AppSettings::default(),
            persist_settings: false,
            bom_category: "all".into(),
            show_welcome_dialog: false,
            show_demo_dialog: false,
            show_settings_dialog: false,
            show_export_dialog: false,
            settings_tab: 0,
            help_tab: 0,
            keymap_search: String::new(),
            keymap_category: None,
            recording_key_action: None,
            export_opts: BlueprintExportOptions::default(),
            welcome_remember_choice: false,
        };
        // Auto-load demo if launched from project root
        if std::path::Path::new("demotracks/mainline-h0.railbench").exists() {
            app.open_layout_path_internal("demotracks/mainline-h0.railbench", false);
            app.undo_stack.clear();
            app.redo_stack.clear();
        }
        app
    }
}

impl RailbenchApp {
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
            "landscape" => Viewer3DEnv::Landscape,
            "minimal" => Viewer3DEnv::Minimal,
            _ => Viewer3DEnv::Hall,
        };
        app.perf_monitor.visible = s.perf_monitor_visible;
        app.app_settings = s;

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
            Viewer3DEnv::Hall => "hall",
            Viewer3DEnv::Landscape => "landscape",
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
        self.color_mode = ColorMode::Types;
        self.show_labels = self.app_settings.show_labels;
        self.show_connectors = self.app_settings.show_connectors;
        self.show_rulers = self.app_settings.show_rulers;
        self.show_wood_plates = self.app_settings.show_wood_plates;
        self.show_right_sidebar = self.app_settings.show_right_sidebar;
        self.rotation_step = self.app_settings.rotation_step;
        self.viewer3d_env = Viewer3DEnv::Hall;
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
        let key = (
            self.layout_version,
            self.show_wood_plates,
            self.viewer3d_env as u8,
            self.dark_mode,
            sc_pts.len(),
        );
        if self.mesh_3d_key != key || self.mesh_3d.is_none() {
            let opts = Scene3DOptions {
                show_wood_plates: self.show_wood_plates,
                env_mode: self.viewer3d_env,
                dark_mode: self.dark_mode,
                short_circuit_points: sc_pts,
            };
            self.mesh_3d = Some(Arc::new(build_scene_mesh_3d(
                &self.layout,
                &self.catalogs,
                &opts,
            )));
            self.mesh_3d_key = key;
            self.mesh_3d_version = self.mesh_3d_version.wrapping_add(1).max(1);
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
            self.power_sim_cache = Some((ver, res));
        }
        &self.power_sim_cache.as_ref().unwrap().1
    }

    pub fn focus_world_point(&mut self, wx: f64, wy: f64) {
        self.pending_focus_point = Some(Vec2 { x: wx, y: wy });
        self.is_3d_mode = false;
    }

    pub fn push_undo(&mut self) {
        self.undo_stack.push(self.layout.clone());
        if self.undo_stack.len() > 80 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.mark_layout_dirty();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.layout.clone());
            self.layout = prev;
            self.selected.clear();
            self.mark_layout_dirty();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.layout.clone());
            self.layout = next;
            self.selected.clear();
            self.mark_layout_dirty();
        }
    }

    pub fn new_empty_layout(&mut self) {
        self.push_undo();
        self.layout = make_empty_layout();
        self.selected.clear();
        self.pending_fit_view = true;
        self.status_message = Some("New layout created".into());
    }

    pub fn open_layout_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RAILbench Layout", &["railbench", "gleisplan", "json"])
            .pick_file()
        {
            if let Some(s) = path.to_str() {
                self.open_layout_path(s);
            }
        }
    }

    pub fn open_layout_path(&mut self, path: &str) {
        self.open_layout_path_internal(path, true);
    }

    fn open_layout_path_internal(&mut self, path: &str, record_recent: bool) {
        match std::fs::read_to_string(path) {
            Ok(raw) => match load_layout_json(&raw) {
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
                    if let Some(models) = &loaded.models {
                        for m in models {
                            if m.file_data.is_some()
                                && !self.custom_models_library.iter().any(|c| c.name == m.name)
                            {
                                self.custom_models_library.push(m.clone());
                            }
                        }
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
            },
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

    pub fn place_catalog_entry(&mut self, entry: &CatalogEntry) {
        self.push_undo();
        let id = format!("piece-{}", self.next_piece_seq);
        self.next_piece_seq += 1;

        // If a single piece of the SAME type (track-to-track or road-to-road) is selected with an open connector, auto-snap!
        let new_is_road = entry.is_road == Some(true) || crate::catalog::is_road_id(&self.catalogs, &entry.id);
        let mut tr = Transform { x: 0.0, y: 0.0, rotation: 0.0 };
        if self.selected.len() == 1 {
            if let Some(sel_id) = self.selected.iter().next() {
                if let Some(sel_p) = self.layout.pieces.iter().find(|p| &p.id == sel_id) {
                    let sel_is_road = crate::catalog::is_road_id(&self.catalogs, &sel_p.catalog_id);
                    if sel_is_road == new_is_road {
                        if let Some(open_c) = sel_p.connectors.iter().rev().find(|c| c.connected_to.is_none()) {
                            let locals = crate::catalog::local_connectors(entry);
                            let loc0 = &locals[0];
                            let rot = norm_angle_deg(open_c.angle + 180.0 - loc0.angle);
                            let ro = rotate(Vec2 { x: loc0.x, y: loc0.y }, rot);
                            tr = Transform {
                                x: open_c.x - ro.x,
                                y: open_c.y - ro.y,
                                rotation: rot,
                            };
                        }
                    }
                }
            }
        }

        let piece = create_piece(id.clone(), entry, tr, self.layout.active_layer_id.clone());
        self.layout.pieces.push(piece);
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.selected.clear();
        self.selected.insert(id);
        self.last_placed_catalog_id = Some(entry.id.clone());
        self.mark_layout_dirty();
    }

    pub fn place_catalog_entry_at(&mut self, entry: &CatalogEntry, world_pos: Vec2, snap_radius: f64) {
        self.push_undo();
        let id = format!("piece-{}", self.next_piece_seq);
        self.next_piece_seq += 1;

        // Center the piece on the drop position
        let locals = crate::catalog::local_connectors(entry);
        let (cx, cy) = if locals.is_empty() {
            (0.0, 0.0)
        } else {
            let sx: f64 = locals.iter().map(|c| c.x).sum();
            let sy: f64 = locals.iter().map(|c| c.y).sum();
            (sx / locals.len() as f64, sy / locals.len() as f64)
        };
        let tr = Transform {
            x: world_pos.x - cx,
            y: world_pos.y - cy,
            rotation: 0.0,
        };
        let piece = create_piece(id.clone(), entry, tr, self.layout.active_layer_id.clone());
        self.layout.pieces.push(piece);
        self.selected.clear();
        self.selected.insert(id);
        self.last_placed_catalog_id = Some(entry.id.clone());

        if self.snapping_enabled
            && !crate::canvas2d::snap_selected_pieces(&mut self.layout, &self.catalogs, &self.selected, snap_radius)
        {
            crate::building_snap::snap_selected_pieces_to_buildings(
                &mut self.layout,
                &self.catalogs,
                &self.selected,
                snap_radius,
            );
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.mark_layout_dirty();
    }

    pub fn rotate_selected(&mut self, delta_deg: f64) {
        if self.selected.is_empty() {
            return;
        }
        self.push_undo();
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) {
                let px = p.transform.x;
                let py = p.transform.y;
                p.transform.rotation = norm_angle_deg(p.transform.rotation + delta_deg);
                for c in &mut p.connectors {
                    let rel = rotate(Vec2 { x: c.x - px, y: c.y - py }, delta_deg);
                    c.x = px + rel.x;
                    c.y = py + rel.y;
                    c.angle = norm_angle_deg(c.angle + delta_deg);
                    c.connected_to = None;
                }
            }
        }
        if let Some(models) = &mut self.layout.models {
            for m in models {
                if self.selected.contains(&m.id) {
                    m.transform.rotation = norm_angle_deg(m.transform.rotation + delta_deg);
                }
            }
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.mark_layout_dirty();
    }

    pub fn delete_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.push_undo();
        self.layout.pieces.retain(|p| !self.selected.contains(&p.id));
        if let Some(models) = &mut self.layout.models {
            models.retain(|m| !self.selected.contains(&m.id));
        }
        self.selected.clear();
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        self.mark_layout_dirty();
    }

    pub fn place_model_instance(&mut self, mut model: Model3D) {
        self.push_undo();
        let id = format!("model-{}", self.next_piece_seq);
        self.next_piece_seq += 1;
        model.id = id.clone();
        model.layer_id = self.layout.active_layer_id.clone();

        if let Some(ref bid) = model.builtin_id {
            if (bid == "engine-shed" || bid == "turntable-trainhall") && self.selected.len() == 1 {
                if let Some(sid) = self.selected.iter().next() {
                    if let Some(sp) = self.layout.pieces.iter().find(|p| &p.id == sid) {
                        if bid == "turntable-trainhall" && sp.connectors.len() >= 6 {
                            let tt_r = crate::catalog::find_entry(&self.catalogs, &sp.catalog_id)
                                .and_then(|e| e.radius.or_else(|| e.length.map(|l| l * 0.5)))
                                .unwrap_or(189.0);
                            let target_dist = tt_r + 6.0 + model.depth * 0.5;
                            model.transform = Transform {
                                x: sp.transform.x + target_dist,
                                y: sp.transform.y,
                                rotation: 270.0,
                            };
                        } else if let Some(c) =
                            sp.connectors.iter().rev().find(|c| c.connected_to.is_none())
                        {
                            let rad = railbench_core::geometry::deg2rad(c.angle);
                            model.transform = Transform {
                                x: c.x + model.width * 0.5 * rad.cos(),
                                y: c.y + model.width * 0.5 * rad.sin(),
                                rotation: c.angle,
                            };
                        }
                    }
                }
            }
        }

        let name = model.name.clone();
        self.layout.models.get_or_insert_with(Vec::new).push(model);
        self.selected.clear();
        self.selected.insert(id);
        if self.snapping_enabled {
            crate::building_snap::snap_selected_models(&mut self.layout, &self.catalogs, &self.selected);
        }
        self.mark_layout_dirty();
        self.status_message = Some(format!("Placed 3D model: {name}"));
    }

    pub fn place_builtin_model(
        &mut self,
        builtin_id: &str,
        name: &str,
        width: f64,
        depth: f64,
        height: f64,
    ) {
        let category = match builtin_id {
            "mountain" => "terrain",
            "engine-shed" | "turntable-trainhall" => "railway",
            _ => "buildings",
        };
        self.place_model_instance(Model3D {
            id: String::new(),
            name: name.to_string(),
            category: category.into(),
            transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width,
            depth,
            height,
            model_type: "builtin".into(),
            builtin_id: Some(builtin_id.to_string()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        });
    }

    pub fn import_custom_model_dialog(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("3D Models (GLB / GLTF)", &["glb", "gltf"])
            .pick_files()
        {
            for path in paths {
                self.import_custom_model_path(&path);
            }
        }
    }

    pub fn import_custom_model_path(&mut self, path: &std::path::Path) {
        match model_tools::import_model_from_path(path) {
            Ok(model) => {
                if !self.custom_models_library.iter().any(|c| c.name == model.name) {
                    self.custom_models_library.push(model.clone());
                }
                self.place_model_instance(model);
            }
            Err(err) => {
                self.status_message = Some(format!("3D import error: {err}"));
            }
        }
    }

    pub fn copy_selected(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        let pieces: Vec<TrackPiece> = self
            .layout
            .pieces
            .iter()
            .filter(|p| self.selected.contains(&p.id))
            .cloned()
            .collect();
        let models: Vec<Model3D> = self
            .layout
            .models
            .as_ref()
            .map(|ms| ms.iter().filter(|m| self.selected.contains(&m.id)).cloned().collect())
            .unwrap_or_default();
        let total = pieces.len() + models.len();
        if total == 0 {
            return false;
        }
        self.clipboard = Some(ClipboardData { pieces, models });
        self.paste_offset_count = 1;
        self.status_message = Some(if self.lang == Lang::De {
            format!("{total} Element(e) kopiert (Strg+C)")
        } else {
            format!("{total} item(s) copied (Ctrl+C)")
        });
        true
    }

    pub fn paste_clipboard(&mut self, custom_offset: Option<Vec2>) -> bool {
        let Some(cb) = self.clipboard.clone() else {
            self.status_message = Some(if self.lang == Lang::De {
                "Zwischenablage ist leer".into()
            } else {
                "Clipboard is empty".into()
            });
            return false;
        };
        if cb.pieces.is_empty() && cb.models.is_empty() {
            return false;
        }

        let offset = match custom_offset {
            Some(off) => off,
            None => {
                let k = self.paste_offset_count as f64;
                self.paste_offset_count += 1;
                Vec2 {
                    x: 40.0 * k,
                    y: 40.0 * k,
                }
            }
        };

        self.push_undo();
        let mut new_ids = Vec::new();
        let mut last_cat_id = None;

        for mut piece in cb.pieces {
            let new_id = format!("piece-{}", self.next_piece_seq);
            self.next_piece_seq += 1;
            piece.id = new_id.clone();
            piece.transform.x += offset.x;
            piece.transform.y += offset.y;
            for c in &mut piece.connectors {
                c.x += offset.x;
                c.y += offset.y;
                c.connected_to = None;
            }
            last_cat_id = Some(piece.catalog_id.clone());
            self.layout.pieces.push(piece);
            new_ids.push(new_id);
        }

        if !cb.models.is_empty() {
            let models = self.layout.models.get_or_insert_with(Vec::new);
            for mut model in cb.models {
                let new_id = format!("model-{}", self.next_piece_seq);
                self.next_piece_seq += 1;
                model.id = new_id.clone();
                model.transform.x += offset.x;
                model.transform.y += offset.y;
                models.push(model);
                new_ids.push(new_id);
            }
        }

        if let Some(cid) = last_cat_id {
            self.last_placed_catalog_id = Some(cid);
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.selected.clear();
        let total = new_ids.len();
        for id in new_ids {
            self.selected.insert(id);
        }
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{total} Element(e) eingefügt (Strg+V)")
        } else {
            format!("{total} item(s) pasted (Ctrl+V)")
        });
        true
    }

    pub fn duplicate_selected(&mut self) -> bool {
        if !self.copy_selected() {
            return false;
        }
        self.paste_clipboard(Some(Vec2 { x: 40.0, y: 40.0 }))
    }

    pub fn select_all(&mut self) {
        self.selected.clear();
        for p in &self.layout.pieces {
            if self.is_layer_editable(p.layer_id.as_deref()) {
                self.selected.insert(p.id.clone());
            }
        }
        if let Some(models) = &self.layout.models {
            for m in models {
                if self.is_layer_editable(m.layer_id.as_deref()) {
                    self.selected.insert(m.id.clone());
                }
            }
        }
        let n = self.selected.len();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{n} Element(e) ausgewählt")
        } else {
            format!("Selected {n} item(s)")
        });
    }

    pub fn select_connected_tracks(&mut self) {
        let mut visited = HashSet::new();
        let mut queue: VecDeque<String> = self.selected.iter().cloned().collect();
        while let Some(cur_id) = queue.pop_front() {
            if !visited.insert(cur_id.clone()) {
                continue;
            }
            if let Some(piece) = self.layout.pieces.iter().find(|p| p.id == cur_id) {
                for conn in &piece.connectors {
                    if let Some(ref cto) = conn.connected_to {
                        if !visited.contains(&cto.piece_id) {
                            queue.push_back(cto.piece_id.clone());
                        }
                    }
                }
            }
        }
        self.selected = visited;
        let n = self.selected.len();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{n} verbundene Gleise ausgewählt")
        } else {
            format!("Selected {n} connected track(s)")
        });
    }

    pub fn select_layer_tracks(&mut self) {
        let target_lid = self
            .layout
            .pieces
            .iter()
            .find(|p| self.selected.contains(&p.id))
            .and_then(|p| p.layer_id.clone())
            .or_else(|| self.layout.active_layer_id.clone())
            .unwrap_or_else(|| "layer-1".into());
        self.selected.clear();
        for p in &self.layout.pieces {
            if p.layer_id.as_deref().unwrap_or("layer-1") == target_lid
                && self.is_layer_editable(p.layer_id.as_deref())
            {
                self.selected.insert(p.id.clone());
            }
        }
        let n = self.selected.len();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{n} Gleise auf Ebene ausgewählt")
        } else {
            format!("Selected {n} track(s) on layer")
        });
    }

    fn is_layer_editable(&self, layer_id: Option<&str>) -> bool {
        let lid = layer_id.unwrap_or("layer-1");
        self.layout
            .layers
            .as_ref()
            .and_then(|ls| ls.iter().find(|l| l.id == lid))
            .map(|l| l.visible && !l.locked)
            .unwrap_or(true)
    }

    pub fn append_track_to_connector(
        &mut self,
        anchor_piece_id: &str,
        connector_idx: usize,
        specific_catalog_id: Option<&str>,
    ) -> bool {
        let Some(anchor) = self.layout.pieces.iter().find(|p| p.id == anchor_piece_id).cloned() else {
            return false;
        };
        let Some(anchor_conn) = anchor.connectors.get(connector_idx).cloned() else {
            return false;
        };
        if anchor_conn.connected_to.is_some() {
            return false;
        }

        let anchor_is_road = crate::catalog::is_road_id(&self.catalogs, &anchor.catalog_id);
        let anchor_entry = crate::catalog::find_entry(&self.catalogs, &anchor.catalog_id);
        let anchor_gauge = anchor_entry.and_then(|e| e.gauge);

        let chosen_entry = specific_catalog_id
            .and_then(|cid| crate::catalog::find_entry(&self.catalogs, cid))
            .or_else(|| {
                self.last_placed_catalog_id
                    .as_deref()
                    .and_then(|cid| crate::catalog::find_entry(&self.catalogs, cid))
                    .filter(|e| {
                        let e_is_road = e.is_road == Some(true)
                            || crate::catalog::is_road_id(&self.catalogs, &e.id);
                        let gauge_ok = match (anchor_gauge, e.gauge) {
                            (Some(ag), Some(eg)) => (ag - eg).abs() < 2.0,
                            _ => true,
                        };
                        e_is_road == anchor_is_road && gauge_ok && e.kind != "turntable"
                    })
            })
            .or(anchor_entry);

        let Some(entry) = chosen_entry.cloned() else {
            return false;
        };

        let locals = crate::catalog::local_connectors(&entry);
        if locals.is_empty() {
            return false;
        }
        let use_end_conn = entry.kind == "curve" && locals.len() >= 2 && connector_idx == 0;
        let moving_local = if use_end_conn { &locals[1] } else { &locals[0] };

        let rot = norm_angle_deg(anchor_conn.angle + 180.0 - moving_local.angle);
        let ro = rotate(Vec2 { x: moving_local.x, y: moving_local.y }, rot);
        let tr = Transform {
            x: anchor_conn.x - ro.x,
            y: anchor_conn.y - ro.y,
            rotation: rot,
        };

        self.push_undo();
        let new_id = format!("piece-{}", self.next_piece_seq);
        self.next_piece_seq += 1;
        let mut piece = create_piece(new_id.clone(), &entry, tr, anchor.layer_id.clone());
        if let Some(el) = anchor.elevation {
            let elev_v = if connector_idx == 0 { el.start } else { el.end };
            if elev_v.abs() > 0.01 {
                piece.elevation = Some(Elevation { start: elev_v, end: elev_v });
            }
        }
        if anchor.has_catenary == Some(true) {
            piece.has_catenary = Some(true);
        }

        self.layout.pieces.push(piece);
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.selected.clear();
        self.selected.insert(new_id);
        self.last_placed_catalog_id = Some(entry.id.clone());
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("Gleis #{} angesetzt", entry.id)
        } else {
            format!("Attached track #{}", entry.id)
        });
        true
    }

    pub fn move_selected_to_layer(&mut self, target_layer_id: &str) {
        if self.selected.is_empty() {
            return;
        }
        self.push_undo();
        let mut moved = 0usize;
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) {
                p.layer_id = Some(target_layer_id.to_string());
                moved += 1;
            }
        }
        if let Some(models) = &mut self.layout.models {
            for m in models {
                if self.selected.contains(&m.id) {
                    m.layer_id = Some(target_layer_id.to_string());
                    moved += 1;
                }
            }
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.mark_layout_dirty();
        let layer_name = self
            .layout
            .layers
            .as_ref()
            .and_then(|ls| ls.iter().find(|l| l.id == target_layer_id))
            .map(|l| l.name.clone())
            .unwrap_or_else(|| target_layer_id.to_string());
        self.status_message = Some(if self.lang == Lang::De {
            format!("{moved} Element(e) auf \"{layer_name}\" verschoben")
        } else {
            format!("Moved {moved} item(s) to \"{layer_name}\"")
        });
    }

    pub fn count_orphaned_items(&self) -> usize {
        let Some(layers) = &self.layout.layers else {
            return 0;
        };
        if layers.is_empty() {
            return 0;
        }
        let valid: HashSet<&str> = layers.iter().map(|l| l.id.as_str()).collect();
        let orphaned_pieces = self
            .layout
            .pieces
            .iter()
            .filter(|p| !valid.contains(p.layer_id.as_deref().unwrap_or("layer-1")))
            .count();
        let orphaned_models = self
            .layout
            .models
            .as_ref()
            .map(|ms| {
                ms.iter()
                    .filter(|m| !valid.contains(m.layer_id.as_deref().unwrap_or("layer-1")))
                    .count()
            })
            .unwrap_or(0);
        orphaned_pieces + orphaned_models
    }

    pub fn reassign_orphaned_items(&mut self) -> usize {
        let valid: HashSet<String> = self
            .layout
            .layers
            .as_ref()
            .map(|ls| ls.iter().map(|l| l.id.clone()).collect())
            .unwrap_or_default();
        if valid.is_empty() {
            return 0;
        }
        let target = self
            .layout
            .active_layer_id
            .clone()
            .filter(|id| valid.contains(id))
            .or_else(|| {
                self.layout
                    .layers
                    .as_ref()
                    .and_then(|ls| ls.first().map(|l| l.id.clone()))
            })
            .unwrap_or_else(|| "layer-1".into());
        if self.count_orphaned_items() == 0 {
            return 0;
        }
        self.push_undo();
        let mut count = 0usize;
        for p in &mut self.layout.pieces {
            let lid = p.layer_id.as_deref().unwrap_or("layer-1");
            if !valid.contains(lid) {
                p.layer_id = Some(target.clone());
                count += 1;
            }
        }
        if let Some(models) = &mut self.layout.models {
            for m in models {
                let lid = m.layer_id.as_deref().unwrap_or("layer-1");
                if !valid.contains(lid) {
                    m.layer_id = Some(target.clone());
                    count += 1;
                }
            }
        }
        update_layout_connections(&mut self.layout, 4.0, 35.0, 1.0);
        crate::catalog::disconnect_cross_type_connections(&mut self.layout, &self.catalogs);
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{count} verwaiste(s) Element(e) neu zugewiesen")
        } else {
            format!("Reassigned {count} orphaned item(s)")
        });
        count
    }

    fn active_board_bounds(&self) -> board_tools::BoardBounds {
        let boards = board_tools::get_baseboards_slice(&self.layout);
        let target = self
            .active_board_id
            .as_deref()
            .and_then(|aid| boards.iter().find(|b| b.id.as_deref() == Some(aid)))
            .or_else(|| boards.first());
        target
            .map(board_tools::get_baseboard_bounds)
            .unwrap_or(board_tools::BoardBounds {
                x: -1200.0,
                y: -600.0,
                width: 2400.0,
                depth: 1200.0,
            })
    }

    pub fn place_mountain_preset(&mut self, preset: &str) {
        self.push_undo();
        let bounds = self.active_board_bounds();
        let existing_count = self
            .layout
            .models
            .as_ref()
            .map_or(0, |ms| ms.iter().filter(|m| m.builtin_id.as_deref() == Some("mountain")).count());
        let id = format!("mtn-{}", self.next_piece_seq);
        self.next_piece_seq += 1;
        let style = self.terrain_brush.style.clone();
        let mtn = terrain_tools::make_mountain_preset_model(
            preset,
            id.clone(),
            existing_count,
            &style,
            bounds,
            self.layout.active_layer_id.clone(),
            self.lang == Lang::De,
        );
        self.layout.models.get_or_insert_with(Vec::new).push(mtn.clone());
        if self.terrain_brush.auto_tunnel {
            terrain_tools::apply_auto_tunnel_for_mountain(&mtn, &mut self.layout);
        }
        self.selected.clear();
        self.selected.insert(id.clone());
        self.active_terrain_id = Some(id);
        self.terrain_brush.set_active(false, None, None);
        self.mark_layout_dirty();
        self.status_message = Some(format!("Placed mountain: {}", mtn.name));
    }

    pub fn place_ground_terrain(&mut self, terrain_type: &str, name: &str, width: f64, depth: f64, height: f64) {
        self.push_undo();
        let bounds = self.active_board_bounds();
        let id = format!("terrain-{}", self.next_piece_seq);
        self.next_piece_seq += 1;
        let models = self.layout.models.get_or_insert_with(Vec::new);
        models.push(Model3D {
            id: id.clone(),
            name: name.to_string(),
            category: "terrain".into(),
            transform: Transform {
                x: (bounds.x + bounds.width * 0.5).round(),
                y: (bounds.y + bounds.depth * 0.5).round(),
                rotation: 0.0,
            },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: self.layout.active_layer_id.clone(),
            width,
            depth,
            height,
            model_type: "builtin".into(),
            builtin_id: Some("ground-terrain".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: Some(terrain_type.to_string()),
            file_data: None,
            file_name: None,
        });
        self.selected.clear();
        self.selected.insert(id.clone());
        self.active_terrain_id = Some(id);
        self.terrain_brush.set_active(false, None, None);
        self.mark_layout_dirty();
        self.status_message = Some(format!("Placed terrain: {name}"));
    }

    pub fn auto_tunnel_all_mountains(&mut self) -> usize {
        self.push_undo();
        let count = terrain_tools::apply_auto_tunnel_all_mountains(&mut self.layout);
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("Auto-Tunnel: {count} Gleis-/Strassenstueck(e) markiert")
        } else {
            format!("Auto-tunnel marked {count} intersecting track/road piece(s)")
        });
        count
    }

    pub fn clear_all_tunnels(&mut self) -> usize {
        self.push_undo();
        let mut cleared = 0usize;
        for p in &mut self.layout.pieces {
            if p.has_tunnel == Some(true) {
                p.has_tunnel = Some(false);
                cleared += 1;
            }
        }
        self.mark_layout_dirty();
        self.status_message = Some(if self.lang == Lang::De {
            format!("{cleared} Tunnel-Markierung(en) entfernt")
        } else {
            format!("Cleared {cleared} tunnel flag(s)")
        });
        cleared
    }

    pub fn center_terrain_item_on_board(&mut self, id: &str) {
        let bounds = self.active_board_bounds();
        let cx = (bounds.x + bounds.width * 0.5).round();
        let cy = (bounds.y + bounds.depth * 0.5).round();
        self.push_undo();
        let mut updated_mtn: Option<Model3D> = None;
        if let Some(models) = &mut self.layout.models {
            if let Some(m) = models.iter_mut().find(|m| m.id == id) {
                m.transform.x = cx;
                m.transform.y = cy;
                if m.builtin_id.as_deref() == Some("mountain") {
                    updated_mtn = Some(m.clone());
                }
            }
        }
        if let Some(m) = updated_mtn {
            if self.terrain_brush.auto_tunnel {
                terrain_tools::apply_auto_tunnel_for_mountain(&m, &mut self.layout);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_feeder(&mut self) {
        if self.selected.is_empty() { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && p.power_feed != Some(true));
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) {
                p.power_feed = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_isolator(&mut self) {
        if self.selected.is_empty() { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && p.is_isolator != Some(true));
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) {
                p.is_isolator = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_catenary(&mut self) {
        if self.selected.is_empty() { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && p.has_catenary != Some(true));
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) {
                p.has_catenary = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_catenary_feeder(&mut self) {
        let has_target = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
        });
        if !has_target { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id)
                && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
                && p.catenary_power_feed != Some(true)
        });
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                p.catenary_power_feed = Some(any_off);
                if any_off {
                    p.has_catenary = Some(true);
                }
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_catenary_isolator(&mut self) {
        let matching_ids: Vec<String> = self
            .layout
            .pieces
            .iter()
            .filter(|p| {
                self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
            })
            .map(|p| p.id.clone())
            .collect();
        if matching_ids.is_empty() { return; }
        self.push_undo();
        if matching_ids.len() == 1 {
            if let Some(p) = self.layout.pieces.iter_mut().find(|p| p.id == matching_ids[0]) {
                if p.catenary_isolator == Some(true) {
                    let cur = p.catenary_isolated_connector_index.unwrap_or(0);
                    if cur + 1 < p.connectors.len() {
                        p.catenary_isolated_connector_index = Some(cur + 1);
                    } else {
                        p.catenary_isolator = Some(false);
                        p.catenary_isolated_connector_index = None;
                    }
                } else {
                    p.catenary_isolator = Some(true);
                    p.catenary_isolated_connector_index = Some(0);
                    p.has_catenary = Some(true);
                }
            }
        } else {
            let any_off = self.layout.pieces.iter().any(|p| {
                self.selected.contains(&p.id)
                    && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
                    && p.catenary_isolator != Some(true)
            });
            for p in &mut self.layout.pieces {
                if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                    p.catenary_isolator = Some(any_off);
                    if any_off {
                        p.catenary_isolated_connector_index = Some(0);
                        p.has_catenary = Some(true);
                    } else {
                        p.catenary_isolated_connector_index = None;
                    }
                }
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_lantern(&mut self) {
        let is_sw = |p: &TrackPiece, cats: &[Catalog]| -> bool {
            crate::catalog::find_entry(cats, &p.catalog_id).map_or(false, |e| is_turnout_kind(&e.kind))
        };
        if !self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && is_sw(p, &self.catalogs)) {
            return;
        }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && is_sw(p, &self.catalogs) && p.has_lantern != Some(true)
        });
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && is_sw(p, &self.catalogs) {
                p.has_lantern = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_decoder(&mut self) {
        let is_sw = |p: &TrackPiece, cats: &[Catalog]| -> bool {
            crate::catalog::find_entry(cats, &p.catalog_id).map_or(false, |e| is_turnout_kind(&e.kind))
        };
        if !self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && is_sw(p, &self.catalogs)) {
            return;
        }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && is_sw(p, &self.catalogs) && p.has_decoder != Some(true)
        });
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && is_sw(p, &self.catalogs) {
                p.has_decoder = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_signal(&mut self) {
        if !self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
        }) {
            return;
        }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id)
                && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
                && p.has_signal != Some(true)
        });
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                p.has_signal = Some(any_off);
                if any_off {
                    if p.signal_side.is_none() {
                        p.signal_side = Some("right".into());
                    }
                    if p.signal_direction.is_none() {
                        p.signal_direction = Some("forward".into());
                    }
                }
            }
        }
        self.mark_layout_dirty();
    }

    pub fn cycle_selected_signal_side(&mut self) {
        let cur = self
            .layout
            .pieces
            .iter()
            .find(|p| self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id))
            .and_then(|p| p.signal_side.as_deref())
            .unwrap_or("right");
        let next = match cur {
            "right" => "left",
            "left" => "both",
            _ => "right",
        };
        if !self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
        }) {
            return;
        }
        self.push_undo();
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                p.has_signal = Some(true);
                p.signal_side = Some(next.into());
            }
        }
        self.mark_layout_dirty();
    }

    pub fn cycle_selected_signal_direction(&mut self) {
        let cur = self
            .layout
            .pieces
            .iter()
            .find(|p| self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id))
            .and_then(|p| p.signal_direction.as_deref())
            .unwrap_or("forward");
        let next = match cur {
            "forward" => "backward",
            "backward" => "both",
            _ => "forward",
        };
        if !self.layout.pieces.iter().any(|p| {
            self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id)
        }) {
            return;
        }
        self.push_undo();
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) && !crate::catalog::is_road_id(&self.catalogs, &p.catalog_id) {
                p.has_signal = Some(true);
                p.signal_direction = Some(next.into());
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_bridge(&mut self) {
        if self.selected.is_empty() { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && p.has_bridge != Some(true));
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) {
                p.has_bridge = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn toggle_selected_tunnel(&mut self) {
        if self.selected.is_empty() { return; }
        self.push_undo();
        let any_off = self.layout.pieces.iter().any(|p| self.selected.contains(&p.id) && p.has_tunnel != Some(true));
        for p in &mut self.layout.pieces {
            if self.selected.contains(&p.id) {
                p.has_tunnel = Some(any_off);
            }
        }
        self.mark_layout_dirty();
    }

    pub fn add_layer(&mut self) {
        self.push_undo();
        let layers = self.layout.layers.get_or_insert_with(Vec::new);
        let idx = layers.len();
        let id = format!("layer-{}", idx + 1);
        let palette_hex = ["#38bdf8", "#f59e0b", "#10b981", "#a855f7", "#ef4444", "#ec4899"];
        layers.push(Layer {
            id: id.clone(),
            name: format!("Ebene {idx}"),
            color: Some(palette_hex[idx % palette_hex.len()].into()),
            visible: true,
            locked: false,
            height_mm: Some((idx as f64) * 80.0),
            wood_plate_mode: None,
            wood_plate_thickness_mm: None,
            wood_plate_style: None,
            plates: None,
        });
        self.layout.active_layer_id = Some(id);
    }
}

impl eframe::App for RailbenchApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame_start = self.perf_monitor.begin_frame();

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
        }

        if let Some(incoming) = self.collab.poll() {
            self.layout = incoming;
            self.layout_version = self.layout_version.wrapping_add(1);
        }
        if self.collab.connected {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
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
            } else if matches!(ext.as_str(), "railbench" | "gleisplan" | "json") {
                if let Some(s) = path.to_str() {
                    self.open_layout_path(s);
                }
            }
        }

        theme::apply_railbench_theme(ctx, self.dark_mode);

        // Dynamic customizable keyboard shortcuts (disabled while recording a new shortcut or typing text)
        if self.recording_key_action.is_none() && !ctx.wants_keyboard_input() {
            let (
                do_undo,
                do_redo,
                do_save,
                do_open,
                do_new_plan,
                do_export_csv,
                do_print_plan,
                do_copy,
                do_paste,
                do_dup,
                do_sel_all,
                do_fit,
                rot_delta,
                do_delete,
                toggle_3d,
                toggle_rulers,
                toggle_labels,
                toggle_snapping,
                toggle_power_sim,
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
                do_esc,
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
                (
                    s.matches_action(KeyAction::Undo, i),
                    s.matches_action(KeyAction::Redo, i),
                    s.matches_action(KeyAction::Save, i),
                    s.matches_action(KeyAction::Open, i),
                    s.matches_action(KeyAction::NewPlan, i),
                    s.matches_action(KeyAction::ExportCsv, i),
                    s.matches_action(KeyAction::PrintPlan, i),
                    copy,
                    paste,
                    s.matches_action(KeyAction::Duplicate, i),
                    s.matches_action(KeyAction::SelectAll, i),
                    s.matches_action(KeyAction::FitView, i),
                    rot,
                    s.matches_action(KeyAction::DeleteSelected, i),
                    s.matches_action(KeyAction::Toggle3D, i),
                    s.matches_action(KeyAction::ToggleRulers, i),
                    s.matches_action(KeyAction::ToggleLabels, i),
                    s.matches_action(KeyAction::ToggleSnapping, i),
                    s.matches_action(KeyAction::TogglePowerSim, i),
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
                    i.key_pressed(egui::Key::Escape),
                )
            });

            if do_undo { self.undo(); }
            if do_redo { self.redo(); }
            if do_save { self.save_layout_dialog(); }
            if do_open { self.open_layout_dialog(); }
            if do_new_plan { self.new_empty_layout(); }
            if do_export_csv { self.export_bom_csv_dialog(); }
            if do_print_plan { self.show_export_dialog = true; }
            if do_copy { self.copy_selected(); }
            if do_paste { self.paste_clipboard(None); }
            if do_dup { self.duplicate_selected(); }
            if do_sel_all { self.select_all(); }
            if do_fit { self.pending_fit_view = true; }
            if let Some(dir) = rot_delta {
                self.rotate_selected(dir * self.rotation_step);
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
            if do_esc {
                self.selected.clear();
                self.terrain_brush.set_active(false, None, None);
                self.measure_mode = false;
                self.measure_points = None;
                self.dragging_catalog_entry = None;
                self.sketcher.stop();
                self.dragging_board = None;
                self.dragging_model_rot = None;
                self.board_snap_guides.clear();
            }
        }

        // Refresh cached power simulation whenever power mode or dialog needs it
        if self.power_sim_enabled || self.show_power_dialog {
            let _ = self.get_or_update_power_sim();
        }

        draw_top_bars(ctx, self);
        draw_left_sidebar(ctx, self);

        let sim_for_canvas = if self.power_sim_enabled {
            self.power_sim_cache.as_ref().map(|(_, s)| s.clone())
        } else {
            None
        };

        let central_resp = egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
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
                    let sc_pts = self.current_short_circuit_points();
                    draw_viewport_3d(
                        ui,
                        &mut self.camera_3d,
                        &self.mesh_3d,
                        self.mesh_3d_version,
                        &self.layout,
                        &mut self.viewer3d_env,
                        &mut self.day_night_active,
                        &mut self.time_of_day,
                        &mut self.day_night_speed,
                        &mut self.show_wood_plates,
                        sc_pts.first().copied(),
                        self.dark_mode,
                        self.lang,
                        dt,
                    );
                    if self.day_night_active && self.viewer3d_env != Viewer3DEnv::Minimal {
                        ctx.request_repaint();
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

                    let res = draw_canvas_2d(
                        ui,
                        &mut self.layout,
                        &self.catalogs,
                        &mut self.camera,
                        &mut self.selected,
                        &mut self.dragging_active,
                        &mut self.drag_last_world,
                        &mut self.dragging_flex_handle,
                        &mut self.marquee_start,
                        &mut self.measure_points,
                        self.dragging_catalog_entry.as_ref(),
                        self.dark_mode,
                        self.color_mode,
                        self.show_labels,
                        self.show_connectors,
                        self.show_rulers,
                        self.snapping_enabled,
                        self.power_sim_enabled,
                        sim_for_canvas.as_ref(),
                        self.measure_mode,
                        self.rotation_step,
                        &mut self.terrain_brush,
                        self.lang == Lang::De,
                        &mut self.next_piece_seq,
                        &mut self.active_board_id,
                        &mut self.dragging_board,
                        &mut self.board_snap_guides,
                        self.show_wood_plates,
                        &mut self.sketcher,
                        &mut self.dragging_model_rot,
                    );
                    if let Some((drop_world, snap_r)) = res.drop_catalog_at {
                        if let Some(entry) = self.dragging_catalog_entry.take() {
                            self.place_catalog_entry_at(&entry, drop_world, snap_r);
                        }
                    }
                    if let Some((pid, cidx)) = res.quick_add_at {
                        self.append_track_to_connector(&pid, cidx, None);
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
                    if res.duplicate_selected {
                        self.duplicate_selected();
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

        // Floating drag badge while dragging a catalog piece (before/outside canvas)
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
            if ctx.input(|i| i.pointer.primary_released()) {
                self.dragging_catalog_entry = None;
            }
        }

        // Floating selection bar at bottom-center of canvas (matches TS selection-bar.ts)
        if !self.is_3d_mode {
            draw_floating_selection_bar(ctx, self);
        }

        // F3 Hardware & Performance Monitor overlay
        if self.perf_monitor.visible {
            if let Some(stress_count) = self.perf_monitor.draw_overlay(
                ctx,
                &self.layout,
                self.mesh_3d.as_deref(),
                self.is_3d_mode,
                self.lang,
            ) {
                self.generate_stress_tracks(stress_count);
            }
            ctx.request_repaint();
        }

        // Frameless window edge/corner resize handles (for Wayland & X11)
        handle_window_resize_borders(ctx);
        self.perf_monitor.end_frame(frame_start);
    }
}

fn handle_window_resize_borders(ctx: &egui::Context) {
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

fn make_empty_layout() -> Layout {
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
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("RAILbench")
            .with_inner_size([1360.0, 860.0])
            .with_decorations(false)
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "RAILbench",
        options,
        Box::new(|cc| {
            init_wgpu_3d_resources(cc);
            let mut app = RailbenchApp::from_saved_settings();
            app.perf_monitor.set_gpu_from_creation_context(cc);
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bare_app() -> RailbenchApp {
        let mut app = RailbenchApp::default();
        app.layout = make_empty_layout();
        app.selected.clear();
        app.undo_stack.clear();
        app.redo_stack.clear();
        app.clipboard = None;
        app.paste_offset_count = 1;
        app
    }

    #[test]
    fn clipboard_copy_paste_and_duplicate_create_fresh_ids_and_offsets() {
        let mut app = make_bare_app();
        let entry = crate::catalog::find_entry(&app.catalogs, "24188").unwrap().clone();
        app.place_catalog_entry(&entry);
        assert_eq!(app.layout.pieces.len(), 1);
        let orig_id = app.layout.pieces[0].id.clone();

        assert!(app.copy_selected());
        assert!(app.paste_clipboard(None));
        assert_eq!(app.layout.pieces.len(), 2);
        let pasted = &app.layout.pieces[1];
        assert_ne!(pasted.id, orig_id);
        assert!((pasted.transform.x - 40.0).abs() < 1e-6);
        assert!((pasted.transform.y - 40.0).abs() < 1e-6);
        assert!(app.selected.contains(&pasted.id));

        assert!(app.duplicate_selected());
        assert_eq!(app.layout.pieces.len(), 3);
        let dup = &app.layout.pieces[2];
        assert!((dup.transform.x - 80.0).abs() < 1e-6);
        assert!((dup.transform.y - 80.0).abs() < 1e-6);
    }

    #[test]
    fn move_selected_to_layer_and_reassign_orphans() {
        let mut app = make_bare_app();
        app.add_layer();
        let layer2_id = app.layout.active_layer_id.clone().unwrap();
        let entry = crate::catalog::find_entry(&app.catalogs, "24188").unwrap().clone();
        app.place_catalog_entry(&entry);
        app.place_builtin_model("station", "Bahnhof", 320.0, 140.0, 130.0);

        app.select_all();
        assert_eq!(app.selected.len(), 2);
        app.move_selected_to_layer("layer-1");
        assert_eq!(app.layout.pieces[0].layer_id.as_deref(), Some("layer-1"));
        assert_eq!(app.layout.models.as_ref().unwrap()[0].layer_id.as_deref(), Some("layer-1"));

        // Orphan items on a deleted layer and verify detection + reassignment
        app.layout.pieces[0].layer_id = Some("deleted-layer".into());
        assert_eq!(app.count_orphaned_items(), 1);
        app.layout.active_layer_id = Some(layer2_id.clone());
        let reassigned = app.reassign_orphaned_items();
        assert_eq!(reassigned, 1);
        assert_eq!(app.count_orphaned_items(), 0);
        assert_eq!(app.layout.pieces[0].layer_id.as_deref(), Some(layer2_id.as_str()));
    }

    #[test]
    fn append_track_to_connector_snaps_and_inherits_props() {
        let mut app = make_bare_app();
        let entry = crate::catalog::find_entry(&app.catalogs, "24188").unwrap().clone();
        app.place_catalog_entry(&entry);
        app.layout.pieces[0].elevation = Some(Elevation { start: 0.0, end: 50.0 });
        app.layout.pieces[0].has_catenary = Some(true);
        let first_id = app.layout.pieces[0].id.clone();

        assert!(app.append_track_to_connector(&first_id, 1, None));
        assert_eq!(app.layout.pieces.len(), 2);
        let added = &app.layout.pieces[1];
        assert_eq!(added.has_catenary, Some(true));
        assert_eq!(added.elevation.unwrap().start, 50.0);
        // Should automatically connect to piece 0 connector 1
        assert!(added.connectors[0].connected_to.is_some());
    }

    #[test]
    fn accessory_toggles_and_catenary_power_sim() {
        let mut app = make_bare_app();
        let sw_entry = crate::catalog::find_entry(&app.catalogs, "24611").unwrap().clone();
        app.place_catalog_entry(&sw_entry);
        let sw_id = app.layout.pieces[0].id.clone();

        app.toggle_selected_lantern();
        app.toggle_selected_decoder();
        app.toggle_selected_signal();
        app.cycle_selected_signal_side();
        app.cycle_selected_signal_direction();
        app.toggle_selected_catenary_feeder();

        let p = &app.layout.pieces[0];
        assert_eq!(p.has_lantern, Some(true));
        assert_eq!(p.has_decoder, Some(true));
        assert_eq!(p.has_signal, Some(true));
        assert_eq!(p.signal_side.as_deref(), Some("left"));
        assert_eq!(p.signal_direction.as_deref(), Some("backward"));
        assert_eq!(p.has_catenary, Some(true));
        assert_eq!(p.catenary_power_feed, Some(true));

        let sim = app.get_or_update_power_sim();
        let res = sim.results.get(&sw_id).unwrap();
        assert!(res.has_catenary);
        assert!(res.catenary_voltage.is_some());

        // Catenary isolator cycles through all 3 turnout connectors then disables
        let n_conns = app.layout.pieces[0].connectors.len();
        app.toggle_selected_catenary_isolator();
        assert_eq!(app.layout.pieces[0].catenary_isolator, Some(true));
        assert_eq!(app.layout.pieces[0].catenary_isolated_connector_index, Some(0));
        for expected in 1..n_conns {
            app.toggle_selected_catenary_isolator();
            assert_eq!(app.layout.pieces[0].catenary_isolated_connector_index, Some(expected));
        }
        app.toggle_selected_catenary_isolator();
        assert_eq!(app.layout.pieces[0].catenary_isolator, Some(false));
    }

    #[test]
    fn bulk_equip_turnouts_sets_and_clears_lanterns_and_decoders() {
        let mut app = make_bare_app();
        let straight = crate::catalog::find_entry(&app.catalogs, "24188").unwrap().clone();
        let sw_entry = crate::catalog::find_entry(&app.catalogs, "24611").unwrap().clone();
        app.place_catalog_entry(&straight);
        app.place_catalog_entry(&sw_entry);

        let equipped = app.bulk_equip_turnouts(true);
        assert_eq!(equipped, 1);
        assert_ne!(app.layout.pieces[0].has_lantern, Some(true));
        assert_eq!(app.layout.pieces[1].has_lantern, Some(true));
        assert_eq!(app.layout.pieces[1].has_decoder, Some(true));

        let cleared = app.bulk_equip_turnouts(false);
        assert_eq!(cleared, 1);
        assert_eq!(app.layout.pieces[1].has_lantern, Some(false));
        assert_eq!(app.layout.pieces[1].has_decoder, Some(false));
    }
}
