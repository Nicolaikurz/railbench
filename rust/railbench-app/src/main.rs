#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod blueprint_colors;
mod board_tools;
mod bridges_3d;
mod building_snap;
mod cad_tools;
mod canvas2d;
mod catalog;
mod catalog_store;
mod collab;
mod export;
mod i18n;
mod icons;
mod inventory_store;
mod model_tools;
mod models3d_mesh;
mod perf_monitor;
mod power_sim;
mod ramp_tools;
mod settings;
mod terrain_tools;
mod theme;
mod trains_3d;
mod tunnels_3d;
mod turntables_3d;
mod ui_panels;
mod vehicle_sim;
mod version;
mod viewer3d;
mod app_actions;
mod app_editing;
mod app_io;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use eframe::egui;
use railbench_core::geometry::{norm_angle_deg, rotate};
use railbench_core::io::{load_layout_json, save_layout_json};
use railbench_core::snapping::update_layout_connections;
use railbench_core::types::{
    BaseboardConfig, Catalog, CatalogEntry, Elevation, Layer, Layout, Model3D, TrackPiece, Transform, Vec2,
};

use crate::canvas2d::{draw_canvas_2d, Camera2D, Canvas2DCache, ColorMode};
use crate::catalog::create_piece;
use crate::collab::CollabClient;
use crate::export::{
    export_bom_csv, export_layout_pdf_with_options, export_layout_svg_with_options,
    export_layout_xml, import_layout_xml, BlueprintExportOptions,
};
use crate::i18n::Lang;
use crate::power_sim::{simulate_layout_power, PowerSimResult};
use crate::ramp_tools::{RampProfile, RampSelectionState, RampShape};
use crate::settings::{AppSettings, KeyAction, KeyCategory, StartupAction};
use crate::ui_panels::{
    default_layer, draw_dialogs, draw_floating_selection_bar, draw_left_sidebar, draw_right_sidebar,
    draw_top_bars,
};
use crate::viewer3d::{
    draw_viewport_3d, init_wgpu_3d_resources, release_wgpu_3d_resources, Camera3D, Scene3DBundle,
    Scene3DOptions, Viewer3DEnv,
};

#[derive(Debug, Clone)]
pub struct PowerDraft {
    pub feed_voltage: Option<f64>,
    pub catenary_feed_voltage: Option<f64>,
    pub catenary_circuit_mode: Option<String>,
    pub loco_current_ma: Option<f64>,
    pub loco_count: Option<u32>,
    pub turnout_lantern_current_ma: Option<f64>,
    pub turnout_decoder_current_ma: Option<f64>,
    pub power_sim_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingConfirmAction {
    New,
    Open,
    Demo(String),
    Quit,
}

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

pub fn turnout_state_count(kind: &str, connector_count: usize) -> usize {
    match kind {
        "double-slip" => 2,
        "three-way" => 3,
        _ => connector_count.saturating_sub(1).max(2),
    }
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
    pub catalog_category_filter: String,
    pub sidebar_tab: SidebarTab,
    pub camera: Camera2D,
    pub camera_3d: Camera3D,
    pub is_3d_mode: bool,
    pub scene_3d_bundle: Option<Scene3DBundle>,
    pub gpu_3d_resident: bool,
    pub layout_version: u64,
    pub selected: HashSet<String>,
    pub undo_stack: VecDeque<Layout>,
    pub redo_stack: VecDeque<Layout>,
    pub clipboard: Option<ClipboardData>,
    pub paste_offset_count: usize,
    pub last_placed_catalog_id: Option<String>,
    pub dragging_active: bool,
    pub drag_last_world: Option<Vec2>,
    pub dragging_flex_handle: Option<(String, usize)>,
    pub dragging_catalog_entry: Option<CatalogEntry>,
    pub catalog_entry_power_enabled: HashSet<String>,
    pub catalog_entry_isolating_enabled: HashSet<String>,
    pub dark_mode: bool,
    pub lang: Lang,
    pub color_mode: ColorMode,
    pub show_labels: bool,
    pub show_connectors: bool,
    pub show_rulers: bool,
    pub snapping_enabled: bool,
    pub layer_protection: bool,
    pub power_sim_enabled: bool,
    pub power_sim_cache: Option<(u64, Arc<PowerSimResult>)>,
    pub canvas_2d_cache: Canvas2DCache,
    pub applied_theme_dark: Option<bool>,
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
    pub ramp_sel: RampSelectionState,
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
    pub dragging_track_rot: Option<canvas2d::TrackRotationDrag>,
    pub viewer3d_env: Viewer3DEnv,
    pub day_night_active: bool,
    pub time_of_day: f32,
    pub day_night_speed: f32,
    pub last_3d_frame_time: Option<std::time::Instant>,
    pub mesh_3d_key: (u64, bool, u8, bool, u8, usize, u64),
    pub scene_3d_cache: viewer3d::Scene3DCache,
    pub edit_3d: viewer3d::Edit3DState,
    pub cad_tools: cad_tools::CadToolsState,
    pub inventory: inventory_store::TrackInventoryStore,
    pub perf_monitor: perf_monitor::PerfMonitor,
    pub app_settings: AppSettings,
    pub persist_settings: bool,
    pub bom_category: String,
    pub show_welcome_dialog: bool,
    pub show_demo_dialog: bool,
    pub show_settings_dialog: bool,
    pub show_export_dialog: bool,
    pub show_cab_window: bool,
    pub cab_view_layout: u8,
    pub stellwerk_zoom: f32,
    pub stellwerk_pan: [f32; 2],
    pub stellwerk_split_layers: bool,
    pub stellwerk_selected_layer: Option<String>,
    pub stellwerk_layer_views: HashMap<String, (f32, [f32; 2])>,
    pub cab_camera_mode: viewer3d::CabCameraMode,
    pub cab_look_yaw: f32,
    pub cab_look_pitch: f32,
    pub vehicle_sim: vehicle_sim::VehicleSimState,
    pub last_vehicle_step: Option<std::time::Instant>,
    pub settings_tab: u8,
    pub help_tab: u8,
    pub keymap_search: String,
    pub keymap_category: Option<KeyCategory>,
    pub recording_key_action: Option<KeyAction>,
    pub export_opts: BlueprintExportOptions,
    pub welcome_remember_choice: bool,
    pub show_track_props_piece_id: Option<String>,
    pub show_replace_track_piece_id: Option<String>,
    pub replace_track_search: String,
    pub show_layer_dialog: bool,
    pub layer_dialog_edit_id: Option<String>,
    pub layer_dialog_name: String,
    pub layer_dialog_height_mm: f64,
    pub layer_dialog_color: String,
    pub layer_dialog_wood_mode: String,
    pub layer_dialog_wood_thickness: f64,
    pub confirm_delete_layer_id: Option<String>,
    pub sel_bar_drag_undo_pushed: bool,
    pub current_file_path: Option<String>,
    pub saved_layout_version: u64,
    pub last_autosave_version: u64,
    pub last_edit_instant: Option<std::time::Instant>,
    pub show_autosave_recovery: bool,
    pub pending_confirm_action: Option<PendingConfirmAction>,
    pub pending_open_on_start: bool,
    pub allow_close_now: bool,
    pub rotation_snap_locked: bool,
    pub bars_locked: bool,
    pub low_gpu_mode: bool,
    pub toast_message: Option<(String, std::time::Instant)>,
    pub last_window_sync: Option<std::time::Instant>,
    pub last_window_title_state: Option<(String, bool)>,
    pub power_draft: Option<PowerDraft>,
    pub ramp_max_grade_limit: f64,
    pub ramp_target_layer_id: Option<String>,
    pub gen_conn_idx: Option<usize>,
    pub gen_target_layer_id: Option<String>,
    pub gen_slope_up: bool,
    pub gen_arc_angle_deg: f64,
    pub last_collab_cursor_sent: Option<(std::time::Instant, Vec2)>,
    pub last_collab_selection_sent: Vec<String>,
    pub measure_locked: bool,
    pub dragging_model_template: Option<railbench_core::types::Model3D>,
    pub confirm_reset_keys: bool,
}

impl Default for RailbenchApp {
    fn default() -> Self {
        let catalogs = crate::catalog_store::load_all_catalogs();
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
            catalog_category_filter: "all".into(),
            sidebar_tab: SidebarTab::Tracks,
            camera: Camera2D::default(),
            camera_3d: Camera3D::default(),
            is_3d_mode: false,
            scene_3d_bundle: None,
            gpu_3d_resident: false,
            layout_version: 1,
            selected: HashSet::new(),
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            clipboard: None,
            paste_offset_count: 1,
            last_placed_catalog_id: None,
            dragging_active: false,
            drag_last_world: None,
            dragging_flex_handle: None,
            dragging_catalog_entry: None,
            catalog_entry_power_enabled: HashSet::new(),
            catalog_entry_isolating_enabled: HashSet::new(),
            dark_mode: true,
            lang: Lang::De,
            color_mode: ColorMode::Types,
            show_labels: true,
            show_connectors: true,
            show_rulers: true,
            snapping_enabled: true,
            layer_protection: true,
            power_sim_enabled: false,
            power_sim_cache: None,
            canvas_2d_cache: Canvas2DCache::default(),
            applied_theme_dark: None,
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
            ramp_sel: RampSelectionState::default(),
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
            dragging_track_rot: None,
            viewer3d_env: Viewer3DEnv::HallLandscape,
            day_night_active: false,
            time_of_day: 14.0,
            day_night_speed: 1.0,
            last_3d_frame_time: None,
            mesh_3d_key: (u64::MAX, true, 0, true, 0, 0, 0),
            scene_3d_cache: viewer3d::Scene3DCache::default(),
            edit_3d: viewer3d::Edit3DState::default(),
            cad_tools: cad_tools::CadToolsState::default(),
            inventory: inventory_store::TrackInventoryStore::default(),
            perf_monitor: perf_monitor::PerfMonitor::default(),
            app_settings: AppSettings::default(),
            persist_settings: false,
            bom_category: "all".into(),
            show_welcome_dialog: false,
            show_demo_dialog: false,
            show_settings_dialog: false,
            show_export_dialog: false,
            show_cab_window: false,
            cab_view_layout: 2,
            stellwerk_zoom: 1.0,
            stellwerk_pan: [0.0, 0.0],
            stellwerk_split_layers: true,
            stellwerk_selected_layer: None,
            stellwerk_layer_views: HashMap::new(),
            cab_camera_mode: viewer3d::CabCameraMode::Cab,
            cab_look_yaw: 0.0,
            cab_look_pitch: 0.0,
            vehicle_sim: vehicle_sim::VehicleSimState::default(),
            last_vehicle_step: None,
            settings_tab: 0,
            help_tab: 0,
            keymap_search: String::new(),
            keymap_category: None,
            recording_key_action: None,
            export_opts: BlueprintExportOptions::default(),
            welcome_remember_choice: false,
            show_track_props_piece_id: None,
            show_replace_track_piece_id: None,
            replace_track_search: String::new(),
            show_layer_dialog: false,
            layer_dialog_edit_id: None,
            layer_dialog_name: String::new(),
            layer_dialog_height_mm: 0.0,
            layer_dialog_color: "#38bdf8".into(),
            layer_dialog_wood_mode: "none".into(),
            layer_dialog_wood_thickness: 10.0,
            confirm_delete_layer_id: None,
            sel_bar_drag_undo_pushed: false,
            current_file_path: None,
            saved_layout_version: 1,
            last_autosave_version: 1,
            last_edit_instant: None,
            show_autosave_recovery: false,
            pending_confirm_action: None,
            pending_open_on_start: false,
            allow_close_now: false,
            rotation_snap_locked: false,
            bars_locked: false,
            low_gpu_mode: false,
            toast_message: None,
            last_window_sync: None,
            last_window_title_state: None,
            power_draft: None,
            ramp_max_grade_limit: 3.0,
            ramp_target_layer_id: None,
            gen_conn_idx: None,
            gen_target_layer_id: None,
            gen_slope_up: true,
            gen_arc_angle_deg: 180.0,
            last_collab_cursor_sent: None,
            last_collab_selection_sent: Vec::new(),
            measure_locked: false,
            dragging_model_template: None,
            confirm_reset_keys: false,
        };
        // Auto-load demo if launched from project root
        if std::path::Path::new("demotracks/mainline-h0.railbench").exists() {
            app.open_layout_path_internal("demotracks/mainline-h0.railbench", false);
            app.undo_stack.clear();
            app.redo_stack.clear();
        }
        app.saved_layout_version = app.layout_version;
        app.last_autosave_version = app.layout_version;
        app
    }
}

const MAX_UNDO_STEPS: usize = 60;
const MIN_UNDO_STEPS: usize = 8;
const UNDO_MEMORY_BUDGET_BYTES: usize = 48 * 1024 * 1024;

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

fn dedup_model_file_data(models: &mut [Model3D], library: &[Model3D]) {
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
            let dot = if dirty_now { "● " } else { "" };
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
            let (win_sz, is_max) = ctx.input(|i| {
                (
                    i.viewport().inner_rect.map(|r| r.size()),
                    i.viewport().maximized.unwrap_or(false),
                )
            });
            let mut win_changed = self.app_settings.window_maximized != is_max;
            if let Some(sz) = win_sz {
                if !is_max && sz.x >= 500.0 && sz.y >= 400.0 {
                    let prev_w = self.app_settings.window_width.unwrap_or(0.0);
                    let prev_h = self.app_settings.window_height.unwrap_or(0.0);
                    if (prev_w - sz.x).abs() > 8.0 || (prev_h - sz.y).abs() > 8.0 {
                        self.app_settings.window_width = Some(sz.x);
                        self.app_settings.window_height = Some(sz.y);
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
            } else if matches!(ext.as_str(), "railbench" | "gleisplan" | "json" | "xml") {
                if let Some(s) = path.to_str() {
                    self.open_layout_path(s);
                }
            }
        }

        if self.applied_theme_dark != Some(self.dark_mode) {
            theme::apply_railbench_theme(ctx, self.dark_mode);
            self.applied_theme_dark = Some(self.dark_mode);
        }

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
                if self.measure_mode {
                    if self.measure_points.is_some() {
                        self.measure_points = None;
                        self.measure_locked = false;
                    } else {
                        self.measure_mode = false;
                        self.measure_locked = false;
                    }
                } else if self.terrain_brush.active
                    || self.dragging_catalog_entry.is_some()
                    || self.sketcher.active
                    || self.dragging_board.is_some()
                    || self.dragging_model_rot.is_some()
                    || self.dragging_track_rot.is_some()
                    || self.ramp_sel.pick_mode.is_some()
                {
                    self.terrain_brush.set_active(false, None, None);
                    self.dragging_catalog_entry = None;
                    self.sketcher.stop();
                    self.dragging_board = None;
                    self.dragging_model_rot = None;
                    self.dragging_track_rot = None;
                    self.board_snap_guides.clear();
                    self.ramp_sel.pick_mode = None;
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

        // Keep cached power & 2-wire short-circuit simulation up to date (recomputes only when layout_version changes)
        if !self.layout.pieces.is_empty() || self.power_sim_enabled || self.show_power_dialog {
            let _ = self.get_or_update_power_sim();
        }
        if self.cad_tools.nem_check_enabled && self.cad_tools.nem_cache_version != self.layout_version {
            self.cad_tools.nem_violations =
                cad_tools::check_nem_clearances(&self.layout, &self.catalogs);
            self.cad_tools.nem_cache_version = self.layout_version;
        }

        // Advance active locomotives, road vehicles & animated turntable/transfer-table bridges (0% idle CPU when paused)
        let active_drive = (self.vehicle_sim.running && !self.vehicle_sim.vehicles.is_empty())
            || self.vehicle_sim.has_moving_bridges();
        if active_drive {
            let now = std::time::Instant::now();
            let dt = self
                .last_vehicle_step
                .map(|prev| (now - prev).as_secs_f64().clamp(0.0, 0.1))
                .unwrap_or(0.016);
            self.last_vehicle_step = Some(now);
            self.vehicle_sim.step_bridges(dt, &self.layout, &self.catalogs);
            if self.vehicle_sim.running && !self.vehicle_sim.vehicles.is_empty() {
                self.vehicle_sim.step(dt, &self.layout, &self.catalogs);
            }
            ctx.request_repaint();
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
                    let vehicles_mesh = if !self.vehicle_sim.vehicles.is_empty() {
                        let (verts, idxs) = self.vehicle_sim.build_vehicles_mesh_3d(&self.layout.scale);
                        if !idxs.is_empty() {
                            Some(Arc::new(viewer3d::SceneMesh3D::from_raw(verts, idxs)))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
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
                            last_placed_catalog_id: self.last_placed_catalog_id.as_deref(),
                            terrain_brush: &mut self.terrain_brush,
                            snapping_enabled: &mut self.snapping_enabled,
                            layer_protection: self.layer_protection,
                            rotation_step: self.rotation_step,
                        },
                        &mut self.viewer3d_env,
                        &mut self.day_night_active,
                        &mut self.time_of_day,
                        &mut self.day_night_speed,
                        &mut self.show_wood_plates,
                        sc_pts.first().map(|p| Vec2 { x: p[0], y: p[2] }),
                        self.dark_mode,
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
                    if edit_out.add_last_track_requested {
                        let fallback_entry = self
                            .last_placed_catalog_id
                            .as_deref()
                            .and_then(|id| crate::catalog::find_entry(&self.catalogs, id).cloned())
                            .or_else(|| {
                                self.catalogs
                                    .get(self.selected_catalog_idx)
                                    .and_then(|c| c.entries.first().cloned())
                            });
                        if let Some(entry) = fallback_entry {
                            self.place_catalog_entry(&entry);
                        }
                    }
                    if let Some(delta_deg) = edit_out.rotate_delta_deg {
                        self.rotate_selected(delta_deg);
                    }
                    if let Some(elev_mm) = edit_out.elevation_delta_mm {
                        self.push_undo();
                        viewer3d::edit3d::nudge_selection_elevation(
                            &mut self.layout,
                            &self.selected,
                            self.layer_protection,
                            elev_mm,
                        );
                        self.mark_layout_dirty();
                    }
                    if edit_out.delete_requested {
                        self.delete_selected();
                    }
                    if edit_out.close_gap_requested {
                        self.run_close_gap();
                    }
                    if edit_out.parallel_requested {
                        self.run_parallel_tracks();
                    }
                    if edit_out.layout_changed {
                        self.mark_layout_dirty();
                    }
                    if self.day_night_active {
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

                    let ramp_overlay = if self.show_height_dialog && self.height_tab == 1 {
                        Some((&mut self.ramp_sel, self.ramp_start_h, self.ramp_end_h))
                    } else {
                        self.ramp_sel.pick_mode = None;
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
            if ctx.input(|i| i.pointer.primary_released()) {
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
            if ctx.input(|i| i.pointer.primary_released()) {
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
                                    if ui.small_button("✕").clicked() {
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
                                    if ui.small_button("✕").clicked() {
                                        self.measure_mode = false;
                                        self.measure_locked = false;
                                    }
                                });
                            }
                        });
                });
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
                self.scene_3d_bundle.as_ref(),
                self.is_3d_mode,
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

fn main() -> eframe::Result<()> {
    let saved_cfg = AppSettings::load_from_disk();
    let win_w = saved_cfg.window_width.unwrap_or(1360.0).max(800.0);
    let win_h = saved_cfg.window_height.unwrap_or(860.0).max(600.0);
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(version::full_app_title())
        .with_inner_size([win_w, win_h])
        .with_decorations(false)
        .with_resizable(true);
    if saved_cfg.window_maximized {
        viewport = viewport.with_maximized(true);
    }

    // Parse optional CLI flags & env vars for automated Collab Host/Join
    let args: Vec<String> = std::env::args().collect();
    let mut cli_server = std::env::var("RAILBENCH_SERVER")
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);
    let mut cli_port = std::env::var("RAILBENCH_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok());
    let mut cli_password = std::env::var("RAILBENCH_PASSWORD").ok();
    let mut cli_room = std::env::var("RAILBENCH_ROOM").ok();
    let mut cli_connect = std::env::var("RAILBENCH_CONNECT").ok();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--server" => cli_server = true,
            "--port" if i + 1 < args.len() => {
                cli_port = args[i + 1].parse::<u16>().ok();
                i += 1;
            }
            "--password" if i + 1 < args.len() => {
                cli_password = Some(args[i + 1].clone());
                i += 1;
            }
            "--room" if i + 1 < args.len() => {
                cli_room = Some(args[i + 1].clone());
                i += 1;
            }
            "--connect" if i + 1 < args.len() => {
                cli_connect = Some(args[i + 1].clone());
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        multisampling: viewer3d::WINDOW_MSAA_SAMPLES as u16,
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "RAILbench",
        options,
        Box::new(move |cc| {
            init_wgpu_3d_resources(cc, viewer3d::WINDOW_MSAA_SAMPLES);
            let mut app = RailbenchApp::from_saved_settings();
            app.perf_monitor.set_gpu_from_creation_context(cc);
            if let Some(p) = cli_port {
                app.collab.host_port = p;
            }
            if let Some(ref pw) = cli_password {
                app.collab.password = pw.clone();
            }
            if let Some(ref rm) = cli_room {
                app.collab.room_id = rm.clone();
            }
            if cli_server {
                let initial = app.layout.clone();
                app.collab.start_host_and_connect(Some(&initial));
            } else if let Some(ref url) = cli_connect {
                app.collab.server_url = url.clone();
                let initial = app.layout.clone();
                app.collab.connect_with_layout(Some(&initial));
            }
            Ok(Box::new(app))
        }),
    )
}


#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
