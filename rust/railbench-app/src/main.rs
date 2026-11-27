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
mod app_selection_ops;
mod app_shortcuts;
mod app_state;
mod app_update;


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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionScope {
    Tracks,
    Roads,
    Buildings,
    Terrain,
    Boards,
    All,
}

impl SelectionScope {
    pub fn from_sidebar_tab(tab: SidebarTab) -> Self {
        match tab {
            SidebarTab::Tracks => Self::Tracks,
            SidebarTab::Roads => Self::Roads,
            SidebarTab::Models => Self::Buildings,
            SidebarTab::Terrain => Self::Terrain,
            SidebarTab::Board => Self::Boards,
            SidebarTab::Bom => Self::All,
        }
    }

    pub fn allows_piece(self, catalogs: &[Catalog], piece: &TrackPiece) -> bool {
        match self {
            Self::Tracks => !crate::catalog::is_road_id(catalogs, &piece.catalog_id),
            Self::Roads => crate::catalog::is_road_id(catalogs, &piece.catalog_id),
            Self::All => true,
            Self::Buildings | Self::Terrain | Self::Boards => false,
        }
    }

    pub fn allows_model(self, model: &Model3D) -> bool {
        match self {
            Self::Buildings => !crate::terrain_tools::is_terrain_model(model),
            Self::Terrain => crate::terrain_tools::is_terrain_model(model),
            Self::All => true,
            Self::Tracks | Self::Roads | Self::Boards => false,
        }
    }

    pub fn allows_any_piece(self) -> bool {
        matches!(self, Self::Tracks | Self::Roads | Self::All)
    }

    pub fn allows_rail_accessories(self) -> bool {
        matches!(self, Self::Tracks | Self::All)
    }

    pub fn allows_buildings(self) -> bool {
        matches!(self, Self::Buildings | Self::All)
    }

    pub fn allows_terrain(self) -> bool {
        matches!(self, Self::Terrain | Self::All)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CatalogPickerStep {
    #[default]
    Closed,
    Manufacturers,
    Systems(String),
    RoadScales,
}

pub struct RailbenchApp {
    pub layout: Layout,
    pub catalogs: Vec<Catalog>,
    pub selected_catalog_idx: usize,
    pub selected_road_catalog_idx: usize,
    pub catalog_picker_step: CatalogPickerStep,
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
    pub height_dialog_was_open: bool,
    pub show_collab_dialog: bool,
    pub show_help_dialog: bool,
    pub show_right_sidebar: bool,
    pub collab: CollabClient,
    pub batch_elevation_mm: f64,
    pub height_flat_use_custom: bool,
    pub height_flat_layer_id: Option<String>,
    pub height_tab: u8,
    pub ramp_start_h: f64,
    pub ramp_end_h: f64,
    pub ramp_profile: RampProfile,
    pub ramp_auto_fit: bool,
    pub ramp_target_warning: Option<(String, String)>,
    pub ramp_sel: RampSelectionState,
    pub gen_ramp_shape: RampShape,
    pub gen_curve_left: bool,
    pub gen_target_h: f64,
    pub gen_max_grade: f64,
    pub gen_helix_turns: f64,
    pub gen_curve_id: Option<String>,
    pub gen_anchor_piece_id: Option<String>,
    pub gen_pick_mode: bool,
    pub gen_target_layer_choice: String,
    pub gen_preview_pieces: Vec<railbench_core::types::TrackPiece>,
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
    pub cab_cam_smooth: ui_panels::CabCameraSmoother,
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
    pub layer_dialog_focus_name: bool,
    pub layer_dialog_custom_color_open: bool,
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
    pub use_discrete_gpu: bool,
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
            catalog_picker_step: CatalogPickerStep::Closed,
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
            height_dialog_was_open: false,
            show_collab_dialog: false,
            show_help_dialog: false,
            show_right_sidebar: true,
            collab: CollabClient::default(),
            batch_elevation_mm: 0.0,
            height_flat_use_custom: false,
            height_flat_layer_id: None,
            height_tab: 0,
            ramp_start_h: 0.0,
            ramp_end_h: 80.0,
            ramp_profile: RampProfile::Linear,
            ramp_auto_fit: true,
            ramp_target_warning: None,
            ramp_sel: RampSelectionState::default(),
            gen_ramp_shape: RampShape::Straight,
            gen_curve_left: false,
            gen_target_h: 80.0,
            gen_max_grade: 2.8,
            gen_helix_turns: 0.0,
            gen_curve_id: None,
            gen_anchor_piece_id: None,
            gen_pick_mode: false,
            gen_target_layer_choice: String::new(),
            gen_preview_pieces: Vec::new(),
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
            cab_cam_smooth: ui_panels::CabCameraSmoother::default(),
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
            layer_dialog_focus_name: false,
            layer_dialog_custom_color_open: false,
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
            use_discrete_gpu: false,
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
use app_state::*;

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
    } else if let (Some(wx), Some(wy)) = (saved_cfg.window_x, saved_cfg.window_y) {
        viewport = viewport.with_position([wx, wy]);
    }

    // Parse optional CLI file path, CLI flags & env vars (including Electron-compatible aliases)
    let args: Vec<String> = std::env::args().collect();
    let cli = settings::parse_cli_args(&args);
    let cli_server = cli.server
        || std::env::var("RAILBENCH_SERVER")
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
    let cli_port = cli.port.or_else(|| {
        std::env::var("RAILBENCH_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
    });
    let cli_password = cli
        .password
        .or_else(|| std::env::var("RAILBENCH_PASSWORD").ok());
    let cli_room = cli.room.or_else(|| std::env::var("RAILBENCH_ROOM").ok());
    let cli_connect = cli
        .connect
        .or_else(|| std::env::var("RAILBENCH_CONNECT").ok());
    let cli_file_path = cli.file_path;

    let mut wgpu_options = eframe::egui_wgpu::WgpuConfiguration::default();
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(ref mut create_new) = wgpu_options.wgpu_setup {
        create_new.power_preference = if saved_cfg.use_discrete_gpu {
            eframe::egui_wgpu::wgpu::PowerPreference::HighPerformance
        } else {
            eframe::egui_wgpu::wgpu::PowerPreference::LowPower
        };
    }

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        multisampling: viewer3d::WINDOW_MSAA_SAMPLES as u16,
        wgpu_options,
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
            if let Some(ref fpath) = cli_file_path {
                if std::path::Path::new(fpath).exists() {
                    app.open_layout_path(fpath);
                    app.undo_stack.clear();
                    app.redo_stack.clear();
                    app.show_welcome_dialog = false;
                    app.show_autosave_recovery = false;
                }
            }
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
