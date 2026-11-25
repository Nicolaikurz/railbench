mod board_tools;
mod bridges_3d;
mod building_snap;
mod canvas2d;
mod catalog;
mod catalog_store;
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
mod trains_3d;
mod tunnels_3d;
mod turntables_3d;
mod ui_panels;
mod viewer3d;
mod app_actions;
mod app_editing;
mod app_io;

use std::collections::{HashSet, VecDeque};
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
    pub viewer3d_env: Viewer3DEnv,
    pub day_night_active: bool,
    pub time_of_day: f32,
    pub day_night_speed: f32,
    pub last_3d_frame_time: Option<std::time::Instant>,
    pub mesh_3d_key: (u64, bool, u8, bool, u8, usize),
    pub scene_3d_cache: viewer3d::Scene3DCache,
    pub train_3d: trains_3d::Train3DState,
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
            viewer3d_env: Viewer3DEnv::HallLandscape,
            day_night_active: false,
            time_of_day: 14.0,
            day_night_speed: 1.0,
            last_3d_frame_time: None,
            mesh_3d_key: (u64::MAX, true, 0, true, 0, 0),
            scene_3d_cache: viewer3d::Scene3DCache::default(),
            train_3d: trains_3d::Train3DState::default(),
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
                self.ramp_sel.pick_mode = None;
            }
        }

        // Keep cached power & 2-wire short-circuit simulation up to date (recomputes only when layout_version changes)
        if !self.layout.pieces.is_empty() || self.power_sim_enabled || self.show_power_dialog {
            let _ = self.get_or_update_power_sim();
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
                    if self.train_3d.active {
                        self.train_3d.update(&self.layout, &self.catalogs, dt, &mut self.camera_3d);
                    }
                    let trains_mesh = if self.train_3d.active {
                        self.train_3d.build_mesh(&self.layout, &self.catalogs)
                    } else {
                        None
                    };
                    let mut train_toggle = self.train_3d.active;
                    let sc_pts = self.current_short_circuit_points();
                    draw_viewport_3d(
                        ui,
                        &mut self.camera_3d,
                        &self.scene_3d_bundle,
                        trains_mesh,
                        &mut train_toggle,
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
                    if train_toggle && !self.train_3d.active {
                        self.train_3d.enter_mode(&self.layout, &self.catalogs, &mut self.camera_3d);
                    } else if !train_toggle && self.train_3d.active {
                        self.train_3d.active = false;
                    }
                    if self.day_night_active || self.train_3d.needs_repaint() {
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
                        self.dragging_catalog_entry.as_ref(),
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
                        ramp_overlay,
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
        if self.is_3d_mode && self.train_3d.active {
            self.train_3d.draw_hud(
                ctx,
                viewport_rect,
                &self.layout,
                &self.catalogs,
                &mut self.camera_3d,
                self.dark_mode,
                self.lang,
            );
        }
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
                self.scene_3d_bundle.as_ref(),
                self.is_3d_mode,
                self.lang,
            ) {
                self.generate_stress_tracks(stress_count);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
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
        multisampling: viewer3d::WINDOW_MSAA_SAMPLES as u16,
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
            init_wgpu_3d_resources(cc, viewer3d::WINDOW_MSAA_SAMPLES);
            let mut app = RailbenchApp::from_saved_settings();
            app.perf_monitor.set_gpu_from_creation_context(cc);
            Ok(Box::new(app))
        }),
    )
}


#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
