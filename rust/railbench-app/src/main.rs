mod canvas2d;
mod catalog;
mod collab;
mod export;
mod i18n;
mod icons;
mod power_sim;
mod ramp_tools;
mod theme;
mod ui_panels;
mod viewer3d;

use std::collections::HashSet;
use std::sync::Arc;

use eframe::egui;
use railbench_core::geometry::{norm_angle_deg, rotate};
use railbench_core::io::{load_layout_json, save_layout_json};
use railbench_core::snapping::update_layout_connections;
use railbench_core::types::{BaseboardConfig, Catalog, CatalogEntry, Layer, Layout, Model3D, Transform, Vec2};

use crate::canvas2d::{draw_canvas_2d, Camera2D, ColorMode};
use crate::catalog::{builtin_catalogs, create_piece};
use crate::collab::CollabClient;
use crate::export::{export_bom_csv, export_layout_pdf, export_layout_svg};
use crate::i18n::Lang;
use crate::power_sim::{simulate_layout_power, PowerSimResult};
use crate::ramp_tools::{RampProfile, RampShape};
use crate::ui_panels::{
    default_layer, draw_dialogs, draw_floating_selection_bar, draw_left_sidebar, draw_right_sidebar, draw_top_bars,
};
use crate::viewer3d::{build_scene_mesh_3d, draw_viewport_3d, init_wgpu_3d_resources, Camera3D, SceneMesh3D};

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
    pub mountain_brush_active: bool,
    pub mountain_brush_radius: f64,
    pub mountain_brush_height: f64,
    pub active_brush_model_id: Option<String>,
    pub pending_fit_view: bool,
    pub status_message: Option<String>,
    pub next_piece_seq: u64,
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
            mountain_brush_active: false,
            mountain_brush_radius: 180.0,
            mountain_brush_height: 220.0,
            active_brush_model_id: None,
            pending_fit_view: true,
            status_message: None,
            next_piece_seq: 1,
        };
        // Auto-load demo if launched from project root
        if std::path::Path::new("demotracks/mainline-h0.railbench").exists() {
            app.open_layout_path("demotracks/mainline-h0.railbench");
        }
        app
    }
}

impl RailbenchApp {
    pub fn mark_layout_dirty(&mut self) {
        self.layout_version = self.layout_version.wrapping_add(1);
        self.collab.send_layout(&self.layout);
    }

    pub fn export_bom_csv_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name(format!("{}-bom.csv", self.layout.name))
            .save_file()
        {
            let csv = export_bom_csv(&self.layout, &self.catalogs);
            match std::fs::write(&path, csv) {
                Ok(_) => self.status_message = Some(format!("Exported CSV: {}", path.display())),
                Err(e) => self.status_message = Some(format!("CSV export error: {e}")),
            }
        }
    }

    pub fn export_svg_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SVG Blueprint", &["svg"])
            .set_file_name(format!("{}.svg", self.layout.name))
            .save_file()
        {
            let svg = export_layout_svg(&self.layout);
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
            let pdf = export_layout_pdf(&self.layout);
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

    pub fn ensure_mesh_3d(&mut self) {
        if self.mesh_3d_version != self.layout_version || self.mesh_3d.is_none() {
            self.mesh_3d = Some(Arc::new(build_scene_mesh_3d(&self.layout, &self.catalogs)));
            self.mesh_3d_version = self.layout_version;
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
        match std::fs::read_to_string(path) {
            Ok(raw) => match load_layout_json(&raw) {
                Ok(mut loaded) => {
                    if loaded.layers.as_ref().map_or(true, |l| l.is_empty()) {
                        loaded.layers = Some(vec![default_layer()]);
                        loaded.active_layer_id = Some("layer-1".into());
                    }
                    update_layout_connections(&mut loaded, 4.0, 35.0, 1.0);
                    self.push_undo();
                    self.layout = loaded;
                    self.selected.clear();
                    self.pending_fit_view = true;
                    self.status_message = Some(format!("Loaded {path}"));
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
                    Ok(_) => self.status_message = Some(format!("Saved {}", path.display())),
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

        if self.snapping_enabled {
            crate::canvas2d::snap_selected_pieces(&mut self.layout, &self.catalogs, &self.selected, snap_radius);
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

    pub fn place_builtin_model(&mut self, builtin_id: &str, name: &str, width: f64, depth: f64, height: f64) {
        self.push_undo();
        let id = format!("model-{}", self.next_piece_seq);
        self.next_piece_seq += 1;
        let models = self.layout.models.get_or_insert_with(Vec::new);
        models.push(Model3D {
            id,
            name: name.to_string(),
            category: if builtin_id == "mountain" { "terrain".into() } else { "buildings".into() },
            transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: self.layout.active_layer_id.clone(),
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
        self.mark_layout_dirty();
        self.status_message = Some(format!("Placed 3D model: {name}"));
    }

    pub fn place_ground_terrain(&mut self, terrain_type: &str, name: &str, width: f64, depth: f64, height: f64) {
        self.push_undo();
        let id = format!("terrain-{}", self.next_piece_seq);
        self.next_piece_seq += 1;
        let models = self.layout.models.get_or_insert_with(Vec::new);
        models.push(Model3D {
            id,
            name: name.to_string(),
            category: "terrain".into(),
            transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
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
        self.mark_layout_dirty();
        self.status_message = Some(format!("Placed terrain: {name}"));
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
        if let Some(incoming) = self.collab.poll() {
            self.layout = incoming;
            self.layout_version = self.layout_version.wrapping_add(1);
        }
        if self.collab.connected {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }

        theme::apply_railbench_theme(ctx, self.dark_mode);

        // Keyboard shortcuts (matching TS keymap)
        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                if i.modifiers.command && i.key_pressed(egui::Key::Z) {
                    if i.modifiers.shift {
                        self.redo();
                    } else {
                        self.undo();
                    }
                } else if i.modifiers.command && i.key_pressed(egui::Key::Y) {
                    self.redo();
                } else if i.modifiers.command && i.key_pressed(egui::Key::S) {
                    self.save_layout_dialog();
                } else if i.modifiers.command && i.key_pressed(egui::Key::O) {
                    self.open_layout_dialog();
                } else if i.modifiers.command && i.key_pressed(egui::Key::Num0) {
                    self.pending_fit_view = true;
                } else if i.key_pressed(egui::Key::R) {
                    let delta = if i.modifiers.shift { -self.rotation_step } else { self.rotation_step };
                    self.rotate_selected(delta);
                } else if i.key_pressed(egui::Key::F) && !i.modifiers.command {
                    self.toggle_selected_feeder();
                } else if i.key_pressed(egui::Key::I) && !i.modifiers.command {
                    self.toggle_selected_isolator();
                } else if i.key_pressed(egui::Key::O) && !i.modifiers.command {
                    self.toggle_selected_catenary();
                } else if i.key_pressed(egui::Key::B) && !i.modifiers.command {
                    self.toggle_selected_bridge();
                } else if i.key_pressed(egui::Key::Num3) {
                    self.toggle_3d();
                } else if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                    self.delete_selected();
                }
            });
        }

        // Refresh cached power simulation whenever power mode or dialog needs it
        if self.power_sim_enabled || self.show_power_dialog {
            let _ = self.get_or_update_power_sim();
        }

        draw_top_bars(ctx, self);
        draw_left_sidebar(ctx, self);
        draw_right_sidebar(ctx, self);
        draw_dialogs(ctx, self);

        let sim_for_canvas = if self.power_sim_enabled {
            self.power_sim_cache.as_ref().map(|(_, s)| s.clone())
        } else {
            None
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                if self.is_3d_mode {
                    if self.pending_fit_view {
                        self.camera_3d.fit_layout(&self.layout);
                        self.pending_fit_view = false;
                    }
                    self.ensure_mesh_3d();
                    draw_viewport_3d(
                        ui,
                        &mut self.camera_3d,
                        &self.mesh_3d,
                        self.mesh_3d_version,
                        self.dark_mode,
                    );
                } else {
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
                        self.mountain_brush_active,
                        self.mountain_brush_radius,
                        self.mountain_brush_height,
                        &mut self.active_brush_model_id,
                        &mut self.next_piece_seq,
                    );
                    if let Some((drop_world, snap_r)) = res.drop_catalog_at {
                        if let Some(entry) = self.dragging_catalog_entry.take() {
                            self.place_catalog_entry_at(&entry, drop_world, snap_r);
                        }
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
                    if res.toggle_feeder {
                        self.toggle_selected_feeder();
                    }
                    if res.toggle_isolator {
                        self.toggle_selected_isolator();
                    }
                    if res.toggle_catenary {
                        self.toggle_selected_catenary();
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

        // Frameless window edge/corner resize handles (for Wayland & X11)
        handle_window_resize_borders(ctx);
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
        baseboard: Some(BaseboardConfig {
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
        }),
        baseboards: None,
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
            Ok(Box::new(RailbenchApp::default()))
        }),
    )
}
