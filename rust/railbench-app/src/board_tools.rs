use std::collections::HashMap;

use railbench_core::geometry::{
    deg2rad, rotate, sample_arc_between_connectors, sample_flex_spline_points,
    simplify_cutout_polygon,
};
use railbench_core::types::{
    BaseboardConfig, Catalog, CatalogEntry, Layer, LayerPlate, Layout, TrackPiece, Vec2,
};

use crate::catalog::{find_entry, is_road_id};
use crate::ramp_tools::min_clearance_for_scale;

#[derive(Debug, Clone, Copy)]
pub struct BoardBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub depth: f64,
}

#[derive(Debug, Clone)]
pub struct BaseboardSnapGuide {
    pub is_x: bool,
    pub pos: f64,
    pub from: f64,
    pub to: f64,
}

#[derive(Debug, Clone)]
pub struct BaseboardSnapResult {
    pub x: f64,
    pub y: f64,
    pub snapped_x: bool,
    pub snapped_y: bool,
    pub guides: Vec<BaseboardSnapGuide>,
}

#[derive(Debug, Clone)]
pub struct TrackCutoutPolygon {
    pub points: Vec<Vec2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SketchMode {
    Poly,
    Rect,
    Edit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SketchTarget {
    Table {
        board_id: Option<String>,
        create_new: bool,
    },
    Plate {
        layer_id: String,
        board_id: String,
        plate_id: Option<String>,
        create_new: bool,
    },
}

#[derive(Debug, Clone)]
pub struct BoardSketcherState {
    pub active: bool,
    pub mode: SketchMode,
    pub target: Option<SketchTarget>,
    pub snap_step_mm: f64,
    pub ortho_snap: bool,
    pub points: Vec<Vec2>,
    pub rect_start: Option<Vec2>,
    pub dragging_rect: bool,
    pub edit_points: Vec<Vec2>,
    pub dragging_vertex: Option<usize>,
    pub active_guides: Vec<(Vec2, Vec2)>,
}

impl Default for BoardSketcherState {
    fn default() -> Self {
        Self {
            active: false,
            mode: SketchMode::Poly,
            target: None,
            snap_step_mm: 50.0,
            ortho_snap: true,
            points: Vec::new(),
            rect_start: None,
            dragging_rect: false,
            edit_points: Vec::new(),
            dragging_vertex: None,
            active_guides: Vec::new(),
        }
    }
}

impl BoardSketcherState {
    pub fn reset_drawing(&mut self) {
        self.points.clear();
        self.rect_start = None;
        self.dragging_rect = false;
        self.edit_points.clear();
        self.dragging_vertex = None;
        self.active_guides.clear();
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.target = None;
        self.reset_drawing();
    }

    pub fn start_table_sketch(
        &mut self,
        layout: &Layout,
        mode: SketchMode,
        board_id: Option<String>,
        create_new: bool,
    ) {
        self.active = true;
        self.mode = mode;
        let boards = get_baseboards_slice(layout);
        let chosen_id = board_id.or_else(|| boards.first().and_then(|b| b.id.clone()));
        self.target = Some(SketchTarget::Table {
            board_id: chosen_id.clone(),
            create_new: if mode == SketchMode::Edit { false } else { create_new },
        });
        self.reset_drawing();
        if mode == SketchMode::Edit {
            if let Some(bb) = boards
                .iter()
                .find(|b| b.id == chosen_id)
                .or_else(|| boards.first())
            {
                self.edit_points = get_baseboard_polygon(bb);
            }
        }
    }

    pub fn start_plate_sketch(
        &mut self,
        layout: &Layout,
        mode: SketchMode,
        layer_id: String,
        board_id: String,
        plate_id: Option<String>,
        create_new: bool,
    ) {
        self.active = true;
        self.mode = mode;
        self.target = Some(SketchTarget::Plate {
            layer_id: layer_id.clone(),
            board_id: board_id.clone(),
            plate_id: plate_id.clone(),
            create_new: if mode == SketchMode::Edit {
                false
            } else {
                create_new || plate_id.is_none()
            },
        });
        self.reset_drawing();
        if mode == SketchMode::Edit {
            if let Some(pid) = plate_id {
                let boards = get_baseboards_slice(layout);
                if let Some(plate) = layout
                    .layers
                    .as_ref()
                    .and_then(|ls| ls.iter().find(|l| l.id == layer_id))
                    .and_then(|l| l.plates.as_ref())
                    .and_then(|ps| ps.iter().find(|p| p.id == pid))
                {
                    let target_board = plate
                        .baseboard_id
                        .as_deref()
                        .and_then(|bid| boards.iter().find(|b| b.id.as_deref() == Some(bid)))
                        .or_else(|| boards.iter().find(|b| b.id.as_deref() == Some(&board_id)))
                        .or_else(|| boards.first());
                    self.edit_points = get_layer_plate_polygon(plate, target_board);
                }
            }
        }
    }

    pub fn set_mode(&mut self, layout: &Layout, mode: SketchMode) {
        let Some(t) = self.target.clone() else { return };
        match t {
            SketchTarget::Table { board_id, create_new } => {
                self.start_table_sketch(layout, mode, board_id, create_new);
            }
            SketchTarget::Plate {
                layer_id,
                board_id,
                plate_id,
                create_new,
            } => {
                self.start_plate_sketch(layout, mode, layer_id, board_id, plate_id, create_new);
            }
        }
    }

    pub fn compute_snapped_point(
        &mut self,
        raw: Vec2,
        refs: &[Vec2],
        disable_snap: bool,
    ) -> Vec2 {
        self.active_guides.clear();
        let step = if disable_snap { 0.0 } else { self.snap_step_mm };
        let mut x = if step > 0.0 {
            (raw.x / step).round() * step
        } else {
            raw.x.round()
        };
        let mut y = if step > 0.0 {
            (raw.y / step).round() * step
        } else {
            raw.y.round()
        };

        if self.ortho_snap && !disable_snap && !refs.is_empty() {
            let tol = (step * 0.8).max(25.0);
            let last = refs[refs.len() - 1];
            let first = refs[0];

            if (x - last.x).abs() <= tol {
                x = last.x;
                self.active_guides.push((last, Vec2 { x, y }));
            } else if refs.len() >= 2 && (x - first.x).abs() <= tol {
                x = first.x;
                self.active_guides.push((first, Vec2 { x, y }));
            }

            if (y - last.y).abs() <= tol {
                y = last.y;
                self.active_guides.push((last, Vec2 { x, y }));
            } else if refs.len() >= 2 && (y - first.y).abs() <= tol {
                y = first.y;
                self.active_guides.push((first, Vec2 { x, y }));
            }
        }

        Vec2 { x, y }
    }

    pub fn commit_points(
        &mut self,
        layout: &mut Layout,
        next_seq: &mut u64,
        world_pts: &[Vec2],
        as_rect: bool,
        allow_create: bool,
    ) -> Option<String> {
        if world_pts.len() < 3 {
            return None;
        }
        let target = self.target.clone()?;
        match target {
            SketchTarget::Table {
                board_id,
                create_new,
            } => {
                let target_id = if create_new && allow_create {
                    let id = add_baseboard(layout, next_seq);
                    self.target = Some(SketchTarget::Table {
                        board_id: Some(id.clone()),
                        create_new: false,
                    });
                    id
                } else {
                    ensure_baseboards(layout);
                    board_id
                        .filter(|id| {
                            layout
                                .baseboards
                                .as_ref()
                                .map_or(false, |bs| bs.iter().any(|b| b.id.as_deref() == Some(id)))
                        })
                        .or_else(|| {
                            layout
                                .baseboards
                                .as_ref()
                                .and_then(|bs| bs.first().and_then(|b| b.id.clone()))
                        })
                        .unwrap_or_else(|| "board-1".into())
                };
                if let Some(bs) = &mut layout.baseboards {
                    if let Some(bb) = bs.iter_mut().find(|b| b.id.as_deref() == Some(&target_id)) {
                        apply_polygon_to_table(bb, world_pts, as_rect);
                    }
                }
                sync_primary_baseboard(layout);
                Some(target_id)
            }
            SketchTarget::Plate {
                layer_id,
                board_id,
                plate_id,
                create_new,
            } => {
                let boards = get_baseboards_slice(layout);
                let target_board = boards
                    .iter()
                    .find(|b| b.id.as_deref() == Some(&board_id))
                    .or_else(|| boards.first())
                    .cloned();
                let layers = layout.layers.as_mut()?;
                let layer = layers.iter_mut().find(|l| l.id == layer_id)?;
                let plates = layer.plates.get_or_insert_with(Vec::new);

                let pid = if plate_id.is_none() || (create_new && allow_create) {
                    let id = format!("plate-{}", *next_seq);
                    *next_seq += 1;
                    let idx = plates.len() + 1;
                    let style = target_board
                        .as_ref()
                        .map(|b| b.style.clone())
                        .unwrap_or_else(|| "wood".into());
                    plates.push(LayerPlate {
                        id: id.clone(),
                        name: Some(format!("Platte {idx}")),
                        baseboard_id: target_board.as_ref().and_then(|b| b.id.clone()),
                        shape: Some("rect".into()),
                        custom_polygon: None,
                        triangle_type: None,
                        l_wing_width: None,
                        l_wing_depth: None,
                        l_side: None,
                        u_left_wing_width: None,
                        u_right_wing_width: None,
                        u_back_depth: None,
                        t_bar_depth: None,
                        t_stem_width: None,
                        rotation: Some(0.0),
                        width: 1000.0,
                        depth: 600.0,
                        x: Some(0.0),
                        y: Some(0.0),
                        thickness_mm: Some(layer.wood_plate_thickness_mm.unwrap_or(12.0)),
                        position_mode: Some("custom".into()),
                        style: Some(style),
                    });
                    self.target = Some(SketchTarget::Plate {
                        layer_id: layer_id.clone(),
                        board_id: board_id.clone(),
                        plate_id: Some(id.clone()),
                        create_new: false,
                    });
                    id
                } else {
                    plate_id.unwrap()
                };

                if let Some(plate) = plates.iter_mut().find(|p| p.id == pid) {
                    apply_polygon_to_plate(plate, target_board.as_ref(), world_pts, as_rect);
                }
                sync_wood_mode(layer);
                Some(board_id)
            }
        }
    }
}

pub fn create_default_baseboard(id: String, name: String) -> BaseboardConfig {
    BaseboardConfig {
        id: Some(id),
        name: Some(name),
        enabled: true,
        shape: Some("rect".into()),
        custom_polygon: None,
        triangle_type: None,
        width: 2400.0,
        depth: 1200.0,
        rotation: Some(0.0),
        l_wing_width: Some(800.0),
        l_wing_depth: Some(800.0),
        l_side: Some("left".into()),
        u_left_wing_width: Some(700.0),
        u_right_wing_width: Some(700.0),
        u_back_depth: Some(600.0),
        t_bar_depth: Some(600.0),
        t_stem_width: Some(800.0),
        position_mode: "center".into(),
        custom_x: Some(0.0),
        custom_y: Some(0.0),
        style: "wood".into(),
        corner_radius: Some(12.0),
        show_dimensions: Some(true),
    }
}

pub fn ensure_baseboards(layout: &mut Layout) -> &mut Vec<BaseboardConfig> {
    let needs_init = layout.baseboards.as_ref().map_or(true, |bs| bs.is_empty());
    if needs_init {
        if let Some(mut bb) = layout.baseboard.clone() {
            if bb.id.is_none() {
                bb.id = Some("board-1".into());
            }
            if bb.name.is_none() {
                bb.name = Some("Tisch 1 (Hauptplatte)".into());
            }
            layout.baseboards = Some(vec![bb]);
        } else {
            let def = create_default_baseboard("board-1".into(), "Tisch 1 (Hauptplatte)".into());
            layout.baseboard = Some(def.clone());
            layout.baseboards = Some(vec![def]);
        }
    }
    if let Some(bs) = &mut layout.baseboards {
        for (i, b) in bs.iter_mut().enumerate() {
            if b.id.as_deref().map_or(true, str::is_empty) {
                b.id = Some(format!("board-{}", i + 1));
            }
            if b.name.as_deref().map_or(true, str::is_empty) {
                b.name = Some(format!("Tisch {}", i + 1));
            }
        }
    }
    sync_primary_baseboard(layout);
    layout.baseboards.as_mut().unwrap()
}

pub fn sync_primary_baseboard(layout: &mut Layout) {
    if let Some(first) = layout.baseboards.as_ref().and_then(|bs| bs.first()).cloned() {
        layout.baseboard = Some(first);
    }
}

pub fn get_baseboards_slice(layout: &Layout) -> Vec<BaseboardConfig> {
    if let Some(list) = &layout.baseboards {
        if !list.is_empty() {
            return list.clone();
        }
    }
    if let Some(b) = &layout.baseboard {
        return vec![b.clone()];
    }
    Vec::new()
}

pub fn add_baseboard(layout: &mut Layout, next_seq: &mut u64) -> String {
    let boards = ensure_baseboards(layout);
    let next_num = boards.len() + 1;
    let last_bounds = boards
        .last()
        .map(get_baseboard_bounds)
        .unwrap_or(BoardBounds {
            x: 0.0,
            y: 0.0,
            width: 2400.0,
            depth: 1200.0,
        });
    let id = format!("board-{}", *next_seq);
    *next_seq += 1;
    let mut bb = create_default_baseboard(id.clone(), format!("Tisch {next_num}"));
    bb.width = 1200.0;
    bb.depth = 800.0;
    bb.position_mode = "custom".into();
    bb.custom_x = Some((last_bounds.x + last_bounds.width + 50.0).round());
    bb.custom_y = Some(last_bounds.y.round());
    boards.push(bb);
    sync_primary_baseboard(layout);
    id
}

pub fn duplicate_baseboard(layout: &mut Layout, id: &str, next_seq: &mut u64) -> Option<String> {
    let boards = ensure_baseboards(layout);
    let src = boards.iter().find(|b| b.id.as_deref() == Some(id))?.clone();
    let bounds = get_baseboard_bounds(&src);
    let new_id = format!("board-{}", *next_seq);
    *next_seq += 1;
    let mut clone = src.clone();
    clone.id = Some(new_id.clone());
    clone.name = Some(format!("{} (Kopie)", src.name.as_deref().unwrap_or("Tisch")));
    clone.position_mode = "custom".into();
    clone.custom_x = Some((bounds.x + bounds.width + 50.0).round());
    clone.custom_y = Some(bounds.y.round());
    boards.push(clone);
    sync_primary_baseboard(layout);
    Some(new_id)
}

pub fn remove_baseboard(layout: &mut Layout, id: &str) -> bool {
    let boards = ensure_baseboards(layout);
    let Some(idx) = boards.iter().position(|b| b.id.as_deref() == Some(id)) else {
        return false;
    };
    boards.remove(idx);
    if boards.is_empty() {
        boards.push(create_default_baseboard(
            "board-1".into(),
            "Tisch 1 (Hauptplatte)".into(),
        ));
    }
    sync_primary_baseboard(layout);
    true
}

pub fn apply_shape_defaults(bb: &mut BaseboardConfig) {
    match bb.shape.as_deref().unwrap_or("rect") {
        "l-shape" => {
            bb.l_wing_width.get_or_insert(800.0);
            bb.l_wing_depth.get_or_insert(800.0);
            bb.l_side.get_or_insert_with(|| "left".into());
        }
        "u-shape" => {
            bb.u_left_wing_width.get_or_insert(700.0);
            bb.u_right_wing_width.get_or_insert(700.0);
            bb.u_back_depth.get_or_insert(600.0);
        }
        "t-shape" => {
            bb.t_bar_depth.get_or_insert(600.0);
            bb.t_stem_width.get_or_insert(800.0);
        }
        "triangle" => {
            bb.triangle_type.get_or_insert_with(|| "corner-tl".into());
        }
        "custom" => {
            if bb.custom_polygon.as_ref().map_or(true, |p| p.len() < 3) {
                let w = bb.width.max(200.0);
                let d = bb.depth.max(200.0);
                bb.custom_polygon = Some(vec![
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: w, y: 0.0 },
                    Vec2 { x: w, y: d },
                    Vec2 { x: 0.0, y: d },
                ]);
            }
        }
        _ => {}
    }
}

pub fn apply_plate_shape_defaults(plate: &mut LayerPlate) {
    match plate.shape.as_deref().unwrap_or("rect") {
        "l-shape" => {
            plate.l_wing_width.get_or_insert(800.0);
            plate.l_wing_depth.get_or_insert(800.0);
            plate.l_side.get_or_insert_with(|| "left".into());
        }
        "u-shape" => {
            plate.u_left_wing_width.get_or_insert(700.0);
            plate.u_right_wing_width.get_or_insert(700.0);
            plate.u_back_depth.get_or_insert(600.0);
        }
        "t-shape" => {
            plate.t_bar_depth.get_or_insert(600.0);
            plate.t_stem_width.get_or_insert(800.0);
        }
        "triangle" => {
            plate.triangle_type.get_or_insert_with(|| "corner-tl".into());
        }
        "custom" => {
            if plate.custom_polygon.as_ref().map_or(true, |p| p.len() < 3) {
                let w = plate.width.max(100.0);
                let d = plate.depth.max(100.0);
                plate.custom_polygon = Some(vec![
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: w, y: 0.0 },
                    Vec2 { x: w, y: d },
                    Vec2 { x: 0.0, y: d },
                ]);
            }
        }
        _ => {}
    }
}

pub fn copy_board_shape_to_plate(bb: &BaseboardConfig, plate: &mut LayerPlate) {
    plate.shape = Some(bb.shape.clone().unwrap_or_else(|| "rect".into()));
    plate.width = bb.width;
    plate.depth = bb.depth;
    plate.triangle_type = bb.triangle_type.clone();
    plate.l_wing_width = bb.l_wing_width;
    plate.l_wing_depth = bb.l_wing_depth;
    plate.l_side = bb.l_side.clone();
    plate.u_left_wing_width = bb.u_left_wing_width;
    plate.u_right_wing_width = bb.u_right_wing_width;
    plate.u_back_depth = bb.u_back_depth;
    plate.t_bar_depth = bb.t_bar_depth;
    plate.t_stem_width = bb.t_stem_width;
    plate.custom_polygon = bb.custom_polygon.clone();
}

pub fn make_layer_plate(
    bb: &BaseboardConfig,
    layer: &Layer,
    name: String,
    next_seq: &mut u64,
) -> LayerPlate {
    let id = format!("plate-{}", *next_seq);
    *next_seq += 1;
    let mut plate = LayerPlate {
        id,
        name: Some(name),
        baseboard_id: bb.id.clone(),
        shape: None,
        custom_polygon: None,
        triangle_type: None,
        l_wing_width: None,
        l_wing_depth: None,
        l_side: None,
        u_left_wing_width: None,
        u_right_wing_width: None,
        u_back_depth: None,
        t_bar_depth: None,
        t_stem_width: None,
        rotation: Some(0.0),
        width: bb.width,
        depth: bb.depth,
        x: Some(0.0),
        y: Some(0.0),
        thickness_mm: Some(layer.wood_plate_thickness_mm.unwrap_or(12.0)),
        position_mode: Some("custom".into()),
        style: Some(bb.style.clone()),
    };
    copy_board_shape_to_plate(bb, &mut plate);
    plate
}

pub fn sync_wood_mode(layer: &mut Layer) {
    if layer.height_mm.unwrap_or(0.0).abs() < 1e-6 {
        if layer
            .wood_plate_mode
            .as_deref()
            .map_or(true, |m| m == "none" || m.is_empty())
        {
            layer.wood_plate_mode = Some("table".into());
        }
        return;
    }
    if layer.plates.as_ref().map_or(false, |p| !p.is_empty()) {
        layer.wood_plate_mode = Some("table".into());
    } else if layer.wood_plate_mode.as_deref() == Some("table") {
        layer.wood_plate_mode = Some("none".into());
    }
}

pub fn fit_board_to_tracks(layout: &Layout, bb: &mut BaseboardConfig) -> bool {
    if layout.pieces.is_empty() {
        return false;
    }
    let boards = get_baseboards_slice(layout);
    let mut candidates: Vec<&TrackPiece> = if boards.len() > 1 {
        layout
            .pieces
            .iter()
            .filter(|p| {
                is_point_inside_baseboard(Vec2 { x: p.transform.x, y: p.transform.y }, bb)
                    || p.connectors
                        .iter()
                        .any(|c| is_point_inside_baseboard(Vec2 { x: c.x, y: c.y }, bb))
            })
            .collect()
    } else {
        Vec::new()
    };
    if candidates.is_empty() {
        candidates = layout.pieces.iter().collect();
    }

    let mut pts = Vec::new();
    for p in &candidates {
        pts.push(Vec2 {
            x: p.transform.x,
            y: p.transform.y,
        });
        for c in &p.connectors {
            pts.push(Vec2 { x: c.x, y: c.y });
        }
    }
    if pts.is_empty() {
        return false;
    }

    let rad = deg2rad(bb.rotation.unwrap_or(0.0));
    let cos_n = (-rad).cos();
    let sin_n = (-rad).sin();
    let (mut min_u, mut max_u) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_v, mut max_v) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in pts {
        let u = p.x * cos_n - p.y * sin_n;
        let v = p.x * sin_n + p.y * cos_n;
        min_u = min_u.min(u);
        max_u = max_u.max(u);
        min_v = min_v.min(v);
        max_v = max_v.max(v);
    }

    let padding = 100.0;
    let w = (((max_u - min_u + padding * 2.0) / 50.0).ceil() * 50.0).max(400.0);
    let d = (((max_v - min_v + padding * 2.0) / 50.0).ceil() * 50.0).max(400.0);
    let mid_u = (min_u + max_u) * 0.5;
    let mid_v = (min_v + max_v) * 0.5;
    let cx = mid_u * rad.cos() - mid_v * rad.sin();
    let cy = mid_u * rad.sin() + mid_v * rad.cos();

    bb.width = w;
    bb.depth = d;
    bb.position_mode = "custom".into();
    bb.custom_x = Some((cx - w * 0.5).round());
    bb.custom_y = Some((cy - d * 0.5).round());
    true
}

pub fn apply_polygon_to_table(bb: &mut BaseboardConfig, world_pts: &[Vec2], as_rect: bool) {
    if world_pts.len() < 3 {
        return;
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) =
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for p in world_pts {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    bb.enabled = true;
    bb.position_mode = "custom".into();
    bb.custom_x = Some(min_x.round());
    bb.custom_y = Some(min_y.round());
    bb.width = (max_x - min_x).round().max(100.0);
    bb.depth = (max_y - min_y).round().max(100.0);
    bb.rotation = Some(0.0);
    if as_rect {
        bb.shape = Some("rect".into());
        bb.custom_polygon = None;
    } else {
        bb.shape = Some("custom".into());
        bb.custom_polygon = Some(
            world_pts
                .iter()
                .map(|p| Vec2 {
                    x: ((p.x - min_x) * 10.0).round() / 10.0,
                    y: ((p.y - min_y) * 10.0).round() / 10.0,
                })
                .collect(),
        );
    }
}

pub fn apply_polygon_to_plate(
    plate: &mut LayerPlate,
    target_board: Option<&BaseboardConfig>,
    world_pts: &[Vec2],
    as_rect: bool,
) {
    if world_pts.len() < 3 {
        return;
    }
    let table_bounds = target_board
        .map(get_baseboard_bounds)
        .unwrap_or(BoardBounds {
            x: 0.0,
            y: 0.0,
            width: 2400.0,
            depth: 1200.0,
        });
    let rot = target_board.and_then(|b| b.rotation).unwrap_or(0.0);
    let local_world: Vec<Vec2> = if rot.abs() > 1e-4 {
        let cx = table_bounds.x + table_bounds.width * 0.5;
        let cy = table_bounds.y + table_bounds.depth * 0.5;
        let rad = deg2rad(-rot);
        let cos = rad.cos();
        let sin = rad.sin();
        world_pts
            .iter()
            .map(|pt| {
                let dx = pt.x - cx;
                let dy = pt.y - cy;
                Vec2 {
                    x: cx + dx * cos - dy * sin,
                    y: cy + dx * sin + dy * cos,
                }
            })
            .collect()
    } else {
        world_pts.to_vec()
    };

    let (mut min_x, mut max_x, mut min_y, mut max_y) =
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for p in &local_world {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    plate.position_mode = Some("custom".into());
    plate.x = Some((min_x - table_bounds.x).round());
    plate.y = Some((min_y - table_bounds.y).round());
    plate.width = (max_x - min_x).round().max(50.0);
    plate.depth = (max_y - min_y).round().max(50.0);
    plate.rotation = Some(0.0);
    if as_rect {
        plate.shape = Some("rect".into());
        plate.custom_polygon = None;
    } else {
        plate.shape = Some("custom".into());
        plate.custom_polygon = Some(
            local_world
                .iter()
                .map(|p| Vec2 {
                    x: ((p.x - min_x) * 10.0).round() / 10.0,
                    y: ((p.y - min_y) * 10.0).round() / 10.0,
                })
                .collect(),
        );
    }
}

pub fn get_baseboard_bounds(bb: &BaseboardConfig) -> BoardBounds {
    let (x, y) = match bb.position_mode.as_str() {
        "center" => (-bb.width * 0.5, -bb.depth * 0.5),
        "top-left" => (0.0, 0.0),
        _ => (bb.custom_x.unwrap_or(0.0), bb.custom_y.unwrap_or(0.0)),
    };
    BoardBounds {
        x,
        y,
        width: bb.width,
        depth: bb.depth,
    }
}

pub fn get_baseboard_polygon(bb: &BaseboardConfig) -> Vec<Vec2> {
    let BoardBounds { x, y, width: w, depth: d } = get_baseboard_bounds(bb);
    let shape = bb.shape.as_deref().unwrap_or("rect");
    let mut poly = match shape {
        "triangle" => match bb.triangle_type.as_deref().unwrap_or("corner-tl") {
            "corner-tr" => vec![
                Vec2 { x, y },
                Vec2 { x: x + w, y },
                Vec2 { x: x + w, y: y + d },
            ],
            "corner-br" => vec![
                Vec2 { x: x + w, y },
                Vec2 { x: x + w, y: y + d },
                Vec2 { x, y: y + d },
            ],
            "corner-bl" => vec![
                Vec2 { x, y },
                Vec2 { x: x + w, y: y + d },
                Vec2 { x, y: y + d },
            ],
            "isosceles" => vec![
                Vec2 { x: x + w * 0.5, y },
                Vec2 { x: x + w, y: y + d },
                Vec2 { x, y: y + d },
            ],
            _ => vec![
                Vec2 { x, y },
                Vec2 { x: x + w, y },
                Vec2 { x, y: y + d },
            ],
        },
        "l-shape" => {
            let is_left = bb.l_side.as_deref().unwrap_or("left") == "left";
            let w_wing = bb.l_wing_width.unwrap_or(800.0).min(w * 0.85);
            let d_main = bb.l_wing_depth.unwrap_or(800.0).min(d * 0.85);
            if is_left {
                vec![
                    Vec2 { x, y },
                    Vec2 { x: x + w, y },
                    Vec2 { x: x + w, y: y + d_main },
                    Vec2 { x: x + w_wing, y: y + d_main },
                    Vec2 { x: x + w_wing, y: y + d },
                    Vec2 { x, y: y + d },
                ]
            } else {
                vec![
                    Vec2 { x, y },
                    Vec2 { x: x + w, y },
                    Vec2 { x: x + w, y: y + d },
                    Vec2 { x: x + w - w_wing, y: y + d },
                    Vec2 { x: x + w - w_wing, y: y + d_main },
                    Vec2 { x, y: y + d_main },
                ]
            }
        }
        "u-shape" => {
            let w_left = bb.u_left_wing_width.unwrap_or(700.0).min(w * 0.45);
            let w_right = bb.u_right_wing_width.unwrap_or(700.0).min(w * 0.45);
            let d_back = bb.u_back_depth.unwrap_or(600.0).min(d * 0.85);
            vec![
                Vec2 { x, y },
                Vec2 { x: x + w, y },
                Vec2 { x: x + w, y: y + d },
                Vec2 { x: x + w - w_right, y: y + d },
                Vec2 { x: x + w - w_right, y: y + d_back },
                Vec2 { x: x + w_left, y: y + d_back },
                Vec2 { x: x + w_left, y: y + d },
                Vec2 { x, y: y + d },
            ]
        }
        "t-shape" => {
            let d_bar = bb.t_bar_depth.unwrap_or(600.0).min(d * 0.85);
            let w_stem = bb.t_stem_width.unwrap_or(800.0).min(w * 0.85);
            let stem_left = x + (w - w_stem) * 0.5;
            let stem_right = stem_left + w_stem;
            vec![
                Vec2 { x, y },
                Vec2 { x: x + w, y },
                Vec2 { x: x + w, y: y + d_bar },
                Vec2 { x: stem_right, y: y + d_bar },
                Vec2 { x: stem_right, y: y + d },
                Vec2 { x: stem_left, y: y + d },
                Vec2 { x: stem_left, y: y + d_bar },
                Vec2 { x, y: y + d_bar },
            ]
        }
        "custom" => {
            if let Some(cp) = bb.custom_polygon.as_ref().filter(|p| p.len() >= 3) {
                let mut max_px = 0.0_f64;
                let mut max_py = 0.0_f64;
                for pt in cp {
                    max_px = max_px.max(pt.x);
                    max_py = max_py.max(pt.y);
                }
                let sx = if max_px > 1.0 { w / max_px } else { 1.0 };
                let sy = if max_py > 1.0 { d / max_py } else { 1.0 };
                cp.iter()
                    .map(|pt| Vec2 {
                        x: ((x + pt.x * sx) * 10.0).round() / 10.0,
                        y: ((y + pt.y * sy) * 10.0).round() / 10.0,
                    })
                    .collect()
            } else {
                vec![
                    Vec2 { x, y },
                    Vec2 { x: x + w, y },
                    Vec2 { x: x + w, y: y + d },
                    Vec2 { x, y: y + d },
                ]
            }
        }
        _ => vec![
            Vec2 { x, y },
            Vec2 { x: x + w, y },
            Vec2 { x: x + w, y: y + d },
            Vec2 { x, y: y + d },
        ],
    };

    let rot = bb.rotation.unwrap_or(0.0);
    if rot.abs() > 1e-6 {
        let cx = x + w * 0.5;
        let cy = y + d * 0.5;
        let rad = deg2rad(rot);
        let cos = rad.cos();
        let sin = rad.sin();
        for pt in &mut poly {
            let dx = pt.x - cx;
            let dy = pt.y - cy;
            pt.x = ((cx + dx * cos - dy * sin) * 10.0).round() / 10.0;
            pt.y = ((cy + dx * sin + dy * cos) * 10.0).round() / 10.0;
        }
    }

    poly
}

pub fn layer_plate_as_baseboard(
    plate: &LayerPlate,
    target_board: Option<&BaseboardConfig>,
) -> BaseboardConfig {
    let table_bounds = target_board
        .map(get_baseboard_bounds)
        .unwrap_or(BoardBounds {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            depth: 0.0,
        });
    BaseboardConfig {
        id: Some(plate.id.clone()),
        name: plate.name.clone(),
        enabled: true,
        shape: Some(plate.shape.clone().unwrap_or_else(|| "rect".into())),
        custom_polygon: plate.custom_polygon.clone(),
        triangle_type: plate.triangle_type.clone(),
        width: plate.width.max(50.0),
        depth: plate.depth.max(50.0),
        rotation: Some(plate.rotation.unwrap_or(0.0)),
        l_wing_width: plate.l_wing_width,
        l_wing_depth: plate.l_wing_depth,
        l_side: plate.l_side.clone(),
        u_left_wing_width: plate.u_left_wing_width,
        u_right_wing_width: plate.u_right_wing_width,
        u_back_depth: plate.u_back_depth,
        t_bar_depth: plate.t_bar_depth,
        t_stem_width: plate.t_stem_width,
        position_mode: "custom".into(),
        custom_x: Some(table_bounds.x + plate.x.unwrap_or(0.0)),
        custom_y: Some(table_bounds.y + plate.y.unwrap_or(0.0)),
        style: plate
            .style
            .clone()
            .or_else(|| target_board.map(|b| b.style.clone()))
            .unwrap_or_else(|| "wood".into()),
        corner_radius: Some(8.0),
        show_dimensions: Some(false),
    }
}

pub fn get_layer_plate_polygon(
    plate: &LayerPlate,
    target_board: Option<&BaseboardConfig>,
) -> Vec<Vec2> {
    let dummy = layer_plate_as_baseboard(plate, target_board);
    let mut poly = get_baseboard_polygon(&dummy);
    if let Some(tb) = target_board {
        let rot = tb.rotation.unwrap_or(0.0);
        if rot.abs() > 1e-6 {
            let b = get_baseboard_bounds(tb);
            let cx = b.x + tb.width * 0.5;
            let cy = b.y + tb.depth * 0.5;
            let rad = deg2rad(rot);
            let cos = rad.cos();
            let sin = rad.sin();
            for pt in &mut poly {
                let dx = pt.x - cx;
                let dy = pt.y - cy;
                pt.x = ((cx + dx * cos - dy * sin) * 10.0).round() / 10.0;
                pt.y = ((cy + dx * sin + dy * cos) * 10.0).round() / 10.0;
            }
        }
    }
    poly
}

pub fn is_point_in_polygon(pt: Vec2, poly: &[Vec2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[j];
        let intersect =
            ((a.y > pt.y) != (b.y > pt.y)) && (pt.x < ((b.x - a.x) * (pt.y - a.y)) / (b.y - a.y) + a.x);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub fn is_point_inside_baseboard(pt: Vec2, bb: &BaseboardConfig) -> bool {
    if !bb.enabled {
        return false;
    }
    let poly = get_baseboard_polygon(bb);
    is_point_in_polygon(pt, &poly)
}

fn dist_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    let l2 = vx * vx + vy * vy;
    if l2 < 1e-9 {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    let t = (((p.x - a.x) * vx + (p.y - a.y) * vy) / l2).clamp(0.0, 1.0);
    (p.x - (a.x + t * vx)).hypot(p.y - (a.y + t * vy))
}

pub fn is_point_near_baseboard_edge(pt: Vec2, bb: &BaseboardConfig, tol: f64) -> bool {
    if !bb.enabled {
        return false;
    }
    let poly = get_baseboard_polygon(bb);
    if poly.len() < 3 {
        return false;
    }
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        if dist_to_segment(pt, poly[j], poly[i]) <= tol {
            return true;
        }
        j = i;
    }
    false
}

pub fn compute_baseboard_leg_positions(
    bb: &BaseboardConfig,
    leg_radius: f64,
    base_inset: f64,
) -> Vec<Vec2> {
    let poly = get_baseboard_polygon(bb);
    if poly.len() < 3 {
        return Vec::new();
    }
    let n = poly.len();
    let mut placed: Vec<Vec2> = Vec::new();
    for i in 0..n {
        let p_prev = poly[(i + n - 1) % n];
        let p_curr = poly[i];
        let p_next = poly[(i + 1) % n];

        let v1x = p_curr.x - p_prev.x;
        let v1y = p_curr.y - p_prev.y;
        let l1 = v1x.hypot(v1y);
        let v2x = p_next.x - p_curr.x;
        let v2y = p_next.y - p_curr.y;
        let l2 = v2x.hypot(v2y);
        if l1 < 1e-4 || l2 < 1e-4 {
            continue;
        }
        let u1x = v1x / l1;
        let u1y = v1y / l1;
        let u2x = v2x / l2;
        let u2y = v2y / l2;
        let n1x = -u1y;
        let n1y = u1x;
        let n2x = -u2y;
        let n2y = u2x;

        let min_edge = l1.min(l2);
        let inset = base_inset.min((leg_radius + 5.0).max(min_edge * 0.25));

        let bis_x = n1x + n2x;
        let bis_y = n1y + n2y;
        let bis_len = bis_x.hypot(bis_y);
        let (mut lx, mut ly) = (p_curr.x + bis_x * inset, p_curr.y + bis_y * inset);
        if bis_len > 1e-4 {
            let bis_scale = (inset * 2.8).min((2.0 * inset) / bis_len);
            lx = p_curr.x + (bis_x / bis_len) * bis_scale;
            ly = p_curr.y + (bis_y / bis_len) * bis_scale;
            if !is_point_in_polygon(Vec2 { x: lx, y: ly }, &poly) {
                lx = p_curr.x - (bis_x / bis_len) * bis_scale;
                ly = p_curr.y - (bis_y / bis_len) * bis_scale;
            }
        }
        let pt = Vec2 { x: lx, y: ly };
        if placed.iter().any(|p| (p.x - pt.x).hypot(p.y - pt.y) < 80.0) {
            continue;
        }
        if is_point_in_polygon(pt, &poly) {
            placed.push(pt);
        }
    }
    placed
}

// ============================================================================
//  Concave Polygon + Holes Triangulator (Pure Rust Ear-Clipping + Hole Bridging)
// ============================================================================

pub fn signed_polygon_area(poly: &[Vec2]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0_f64;
    let n = poly.len();
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        sum += a.x * b.y - b.x * a.y;
    }
    sum * 0.5
}

fn clean_polygon_ring(pts: &[Vec2]) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::with_capacity(pts.len());
    for &p in pts {
        if out
            .last()
            .map_or(true, |last| (p.x - last.x).hypot(p.y - last.y) > 0.1)
        {
            out.push(p);
        }
    }
    if out.len() >= 2 && (out[0].x - out.last().unwrap().x).hypot(out[0].y - out.last().unwrap().y) <= 0.1 {
        out.pop();
    }
    out
}

#[inline]
fn cross2(a: Vec2, b: Vec2, c: Vec2) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

#[inline]
fn point_in_tri_ccw(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let c1 = cross2(a, b, p);
    let c2 = cross2(b, c, p);
    let c3 = cross2(c, a, p);
    c1 >= -1e-6 && c2 >= -1e-6 && c3 >= -1e-6
}

fn find_visible_outer_vertex(outer: &[Vec2], m: Vec2) -> Option<usize> {
    let n = outer.len();
    let mut best_x = f64::INFINITY;
    let mut best_edge = None;
    for i in 0..n {
        let a = outer[i];
        let b = outer[(i + 1) % n];
        if (a.y <= m.y && b.y > m.y) || (b.y <= m.y && a.y > m.y) {
            let t = (m.y - a.y) / (b.y - a.y);
            let ix = a.x + t * (b.x - a.x);
            if ix >= m.x - 1e-6 && ix < best_x {
                best_x = ix;
                best_edge = Some(i);
            }
        }
    }
    let e_idx = best_edge?;
    let ea = outer[e_idx];
    let eb = outer[(e_idx + 1) % n];
    let p_idx = if ea.x >= eb.x { e_idx } else { (e_idx + 1) % n };
    let p_vert = outer[p_idx];
    let ix_pt = Vec2 { x: best_x, y: m.y };

    if (p_vert.x - ix_pt.x).hypot(p_vert.y - ix_pt.y) <= 1e-5 {
        return Some(p_idx);
    }

    // Check if triangle (M, I, P) contains reflex vertices; pick minimum angle to +X axis
    let area = cross2(m, ix_pt, p_vert);
    let mut chosen = p_idx;
    let mut min_angle = (p_vert.y - m.y).abs().atan2((p_vert.x - m.x).max(1e-9));
    let mut min_dist = (p_vert.x - m.x).hypot(p_vert.y - m.y);

    for (vi, &v) in outer.iter().enumerate() {
        if vi == p_idx || (v.x - m.x).hypot(v.y - m.y) <= 1e-5 {
            continue;
        }
        let inside = if area >= 0.0 {
            point_in_tri_ccw(v, m, ix_pt, p_vert)
        } else {
            point_in_tri_ccw(v, m, p_vert, ix_pt)
        };
        if !inside {
            continue;
        }
        let prev = outer[(vi + n - 1) % n];
        let next = outer[(vi + 1) % n];
        // Outer polygon is CCW (area > 0), so reflex vertices have cross2(prev, v, next) < 0
        if cross2(prev, v, next) < 1e-5 {
            let ang = (v.y - m.y).abs().atan2((v.x - m.x).max(1e-9));
            let d = (v.x - m.x).hypot(v.y - m.y);
            if ang < min_angle - 1e-6 || ((ang - min_angle).abs() <= 1e-6 && d < min_dist) {
                min_angle = ang;
                min_dist = d;
                chosen = vi;
            }
        }
    }
    Some(chosen)
}

pub fn triangulate_polygon_with_holes(
    outer_raw: &[Vec2],
    holes_raw: &[Vec<Vec2>],
) -> Vec<[Vec2; 3]> {
    let mut outer = clean_polygon_ring(outer_raw);
    if outer.len() < 3 || signed_polygon_area(&outer).abs() < 1e-3 {
        return Vec::new();
    }
    if signed_polygon_area(&outer) < 0.0 {
        outer.reverse();
    }

    // Prepare valid CW holes strictly inside the outer polygon, sorted rightmost first
    let mut holes: Vec<(f64, usize, Vec<Vec2>)> = Vec::new();
    for h_raw in holes_raw {
        let mut h = clean_polygon_ring(h_raw);
        let h_area = signed_polygon_area(&h);
        if h.len() < 3 || h_area.abs() < 1.0 {
            continue;
        }
        if h_area > 0.0 {
            h.reverse();
        }
        if !h.iter().all(|&pt| is_point_in_polygon(pt, &outer)) {
            continue;
        }
        let mut m_idx = 0usize;
        for i in 1..h.len() {
            if h[i].x > h[m_idx].x || ((h[i].x - h[m_idx].x).abs() < 1e-9 && h[i].y < h[m_idx].y) {
                m_idx = i;
            }
        }
        holes.push((h[m_idx].x, m_idx, h));
    }
    holes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    for (_mx, m_idx, h) in holes {
        let m = h[m_idx];
        let Some(v_idx) = find_visible_outer_vertex(&outer, m) else {
            continue;
        };
        let mut combined = Vec::with_capacity(outer.len() + h.len() + 2);
        combined.extend_from_slice(&outer[..=v_idx]);
        for k in 0..h.len() {
            combined.push(h[(m_idx + k) % h.len()]);
        }
        combined.push(m);
        combined.push(outer[v_idx]);
        if v_idx + 1 < outer.len() {
            combined.extend_from_slice(&outer[v_idx + 1..]);
        }
        outer = combined;
    }

    // Ear clipping on merged CCW polygon
    let mut poly = outer;
    let mut tris = Vec::with_capacity(poly.len().saturating_sub(2));
    let mut guard = poly.len() * poly.len() + 32;

    while poly.len() > 3 && guard > 0 {
        guard -= 1;
        let n = poly.len();
        let mut cut_idx: Option<usize> = None;

        for i in 0..n {
            let a = poly[(i + n - 1) % n];
            let b = poly[i];
            let c = poly[(i + 1) % n];
            if cross2(a, b, c) <= 1e-6 {
                continue;
            }
            let mut blocked = false;
            for j in 0..n {
                if j == i || j == (i + n - 1) % n || j == (i + 1) % n {
                    continue;
                }
                let p = poly[j];
                if (p.x - a.x).hypot(p.y - a.y) < 1e-5
                    || (p.x - b.x).hypot(p.y - b.y) < 1e-5
                    || (p.x - c.x).hypot(p.y - c.y) < 1e-5
                {
                    continue;
                }
                if point_in_tri_ccw(p, a, b, c) {
                    blocked = true;
                    break;
                }
            }
            if !blocked {
                cut_idx = Some(i);
                break;
            }
        }

        if let Some(i) = cut_idx {
            let a = poly[(i + n - 1) % n];
            let b = poly[i];
            let c = poly[(i + 1) % n];
            if cross2(a, b, c).abs() > 1e-5 {
                tris.push([a, b, c]);
            }
            poly.remove(i);
        } else {
            // Degenerate/collinear fallback: clip tightest nearly-convex vertex to keep progressing
            let mut best_i = 0usize;
            let mut best_score = f64::INFINITY;
            for i in 0..n {
                let a = poly[(i + n - 1) % n];
                let b = poly[i];
                let c = poly[(i + 1) % n];
                let cr = cross2(a, b, c);
                if cr >= -1e-4 && cr.abs() < best_score {
                    best_score = cr.abs();
                    best_i = i;
                }
            }
            let a = poly[(best_i + n - 1) % n];
            let b = poly[best_i];
            let c = poly[(best_i + 1) % n];
            if cross2(a, b, c).abs() > 1e-4 {
                tris.push([a, b, c]);
            }
            poly.remove(best_i);
        }
    }

    if poly.len() == 3 && cross2(poly[0], poly[1], poly[2]).abs() > 1e-5 {
        tris.push([poly[0], poly[1], poly[2]]);
    }

    tris
}

// ============================================================================
//  Multi-Table Magnetic Snapping (port of src/core/baseboard-snapping.ts)
// ============================================================================

pub fn snap_baseboard(
    cand: &BaseboardConfig,
    cand_x: f64,
    cand_y: f64,
    others: &[BaseboardConfig],
    threshold: f64,
) -> BaseboardSnapResult {
    let w_a = cand.width;
    let d_a = cand.depth;
    let mut best_x = cand_x;
    let mut best_y = cand_y;
    let mut min_diff_x = threshold + 1.0;
    let mut min_diff_y = threshold + 1.0;
    let mut snapped_x = false;
    let mut snapped_y = false;
    let mut guides = Vec::new();

    let enabled_others: Vec<&BaseboardConfig> = others
        .iter()
        .filter(|b| b.id != cand.id && b.enabled)
        .collect();
    if enabled_others.is_empty() {
        return BaseboardSnapResult {
            x: cand_x,
            y: cand_y,
            snapped_x: false,
            snapped_y: false,
            guides,
        };
    }

    for &b in &enabled_others {
        let bb = get_baseboard_bounds(b);
        let left_b = bb.x;
        let right_b = bb.x + bb.width;
        let top_b = bb.y;
        let bot_b = bb.y + bb.depth;
        let mid_xb = bb.x + bb.width * 0.5;
        let mid_yb = bb.y + bb.depth * 0.5;

        for (target_x, diff) in [
            (right_b, (cand_x - right_b).abs()),
            (left_b - w_a, (cand_x + w_a - left_b).abs()),
            (left_b, (cand_x - left_b).abs()),
            (right_b - w_a, (cand_x + w_a - right_b).abs()),
            (mid_xb - w_a * 0.5, (cand_x + w_a * 0.5 - mid_xb).abs()),
        ] {
            if diff < min_diff_x {
                min_diff_x = diff;
                best_x = target_x;
                snapped_x = true;
            }
        }

        for (target_y, diff) in [
            (bot_b, (cand_y - bot_b).abs()),
            (top_b - d_a, (cand_y + d_a - top_b).abs()),
            (top_b, (cand_y - top_b).abs()),
            (bot_b - d_a, (cand_y + d_a - bot_b).abs()),
            (mid_yb - d_a * 0.5, (cand_y + d_a * 0.5 - mid_yb).abs()),
        ] {
            if diff < min_diff_y {
                min_diff_y = diff;
                best_y = target_y;
                snapped_y = true;
            }
        }
    }

    // Polygon vertex-to-vertex snap refinement for L/U/T/custom boards
    let mut temp = cand.clone();
    temp.position_mode = "custom".into();
    temp.custom_x = Some(best_x);
    temp.custom_y = Some(best_y);
    let poly_a = get_baseboard_polygon(&temp);
    'outer: for &b in &enabled_others {
        let poly_b = get_baseboard_polygon(b);
        for &va in &poly_a {
            for &vb in &poly_b {
                let dx = (va.x - vb.x).abs();
                let dy = (va.y - vb.y).abs();
                if dx < threshold && dy < threshold && dx + dy < min_diff_x + min_diff_y {
                    best_x += vb.x - va.x;
                    best_y += vb.y - va.y;
                    snapped_x = true;
                    snapped_y = true;
                    break 'outer;
                }
            }
        }
    }

    if snapped_x {
        for &b in &enabled_others {
            let bb = get_baseboard_bounds(b);
            let right_b = bb.x + bb.width;
            let left_b = bb.x;
            for edge_pos in [left_b, right_b] {
                if (best_x - edge_pos).abs() < 1.5 || (best_x + w_a - edge_pos).abs() < 1.5 {
                    guides.push(BaseboardSnapGuide {
                        is_x: true,
                        pos: edge_pos,
                        from: best_y.min(bb.y) - 60.0,
                        to: (best_y + d_a).max(bb.y + bb.depth) + 60.0,
                    });
                    break;
                }
            }
        }
    }

    if snapped_y {
        for &b in &enabled_others {
            let bb = get_baseboard_bounds(b);
            let top_b = bb.y;
            let bot_b = bb.y + bb.depth;
            for edge_pos in [top_b, bot_b] {
                if (best_y - edge_pos).abs() < 1.5 || (best_y + d_a - edge_pos).abs() < 1.5 {
                    guides.push(BaseboardSnapGuide {
                        is_x: false,
                        pos: edge_pos,
                        from: best_x.min(bb.x) - 60.0,
                        to: (best_x + w_a).max(bb.x + bb.width) + 60.0,
                    });
                    break;
                }
            }
        }
    }

    BaseboardSnapResult {
        x: best_x.round(),
        y: best_y.round(),
        snapped_x,
        snapped_y,
        guides,
    }
}

// ============================================================================
//  Table & Layer Plate Track Cutouts (port of src/core/table-cutout.ts)
// ============================================================================

pub fn get_track_clearance_width(entry: Option<&CatalogEntry>) -> f64 {
    let Some(e) = entry else { return 54.0 };
    if e.is_road == Some(true) {
        return e.road_width.unwrap_or(60.0) + 16.0;
    }
    let gauge = e.gauge.unwrap_or(16.5);
    (gauge * 3.2).round().max(26.0)
}

fn piece_passes_through_deck(
    piece: &TrackPiece,
    layer_map: &HashMap<&str, &Layer>,
    clearance: f64,
    deck_elev: f64,
    deck_thickness: f64,
) -> (bool, f64, f64) {
    let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
    let layer = layer_map.get(lid).copied();
    if let Some(l) = layer {
        if !l.visible {
            return (false, 0.0, 0.0);
        }
    }
    let base_h = layer.and_then(|l| l.height_mm).unwrap_or(0.0);
    let ps = piece.elevation.map_or(0.0, |e| e.start);
    let pe = piece.elevation.map_or(ps, |e| e.end);
    let start_h = base_h + ps;
    let end_h = base_h + pe;
    let min_elev = start_h.min(end_h);
    let max_elev = start_h.max(end_h);

    let goes_below = min_elev < deck_elev - 0.1;
    let is_sloped = (max_elev - min_elev).abs() >= 0.5;
    let intersects_slab = max_elev > deck_elev - deck_thickness;
    let in_zone = max_elev > deck_elev - clearance + 0.1 && (is_sloped || intersects_slab);
    (goes_below && in_zone, min_elev, max_elev)
}

fn piece_centerline_clipped(piece: &TrackPiece, from_c: usize, to_c: usize, t0: f64, t1: f64) -> Vec<Vec2> {
    if let (Some(offsets), Some(chord)) = (&piece.custom_offsets, piece.custom_chord_dist) {
        if offsets.len() >= 3 {
            let flat = sample_flex_spline_points(chord, offsets[0], offsets[1], offsets[2], 16);
            let n = flat.len() / 5;
            let mut pts = Vec::new();
            for k in 0..n {
                let u = k as f64 / (n.saturating_sub(1).max(1)) as f64;
                if u + 0.02 < t0 || u - 0.02 > t1 {
                    continue;
                }
                let rp = rotate(Vec2 { x: flat[k * 5], y: flat[k * 5 + 1] }, piece.transform.rotation);
                pts.push(Vec2 {
                    x: piece.transform.x + rp.x,
                    y: piece.transform.y + rp.y,
                });
            }
            if pts.len() >= 2 {
                return pts;
            }
        }
    }

    let full = if piece.connectors.len() >= 2 {
        let c0 = piece.connectors.get(from_c).unwrap_or(&piece.connectors[0]);
        let c1 = piece.connectors.get(to_c).unwrap_or(&piece.connectors[piece.connectors.len() - 1]);
        sample_arc_between_connectors(c0.x, c0.y, c0.angle, c1.x, c1.y, c1.angle, 12)
    } else {
        vec![Vec2 {
            x: piece.transform.x,
            y: piece.transform.y,
        }]
    };
    if full.len() < 2 {
        return full;
    }

    // Arc-length clip between t0 and t1
    let mut lens = vec![0.0_f64; full.len()];
    for i in 1..full.len() {
        lens[i] = lens[i - 1] + (full[i].x - full[i - 1].x).hypot(full[i].y - full[i - 1].y);
    }
    let total = *lens.last().unwrap_or(&0.0);
    if total < 1e-4 {
        return full;
    }
    let d0 = t0.clamp(0.0, 1.0) * total;
    let d1 = t1.clamp(t0, 1.0) * total;
    let sample_at = |d: f64| -> Vec2 {
        for i in 1..full.len() {
            if lens[i] >= d - 1e-6 || i + 1 == full.len() {
                let span = (lens[i] - lens[i - 1]).max(1e-9);
                let u = ((d - lens[i - 1]) / span).clamp(0.0, 1.0);
                return Vec2 {
                    x: full[i - 1].x + (full[i].x - full[i - 1].x) * u,
                    y: full[i - 1].y + (full[i].y - full[i - 1].y) * u,
                };
            }
        }
        full[full.len() - 1]
    };

    let steps = 8usize;
    let mut out = Vec::with_capacity(steps + 1);
    for k in 0..=steps {
        let u = k as f64 / steps as f64;
        out.push(sample_at(d0 + (d1 - d0) * u));
    }
    out
}

pub fn compute_table_track_cutouts(
    layout: &Layout,
    catalogs: &[Catalog],
    board: &BaseboardConfig,
    deck_elev: f64,
    deck_thickness: f64,
) -> Vec<TrackCutoutPolygon> {
    if !board.enabled || layout.pieces.is_empty() {
        return Vec::new();
    }
    let bounds = get_baseboard_bounds(board);
    let clearance = min_clearance_for_scale(&layout.scale);
    let mut layer_map: HashMap<&str, &Layer> = HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            layer_map.insert(l.id.as_str(), l);
        }
    }

    let mut qualifying: Vec<&TrackPiece> = Vec::new();
    for p in &layout.pieces {
        let (passes, _, _) = piece_passes_through_deck(p, &layer_map, clearance, deck_elev, deck_thickness);
        if !passes {
            continue;
        }
        let near = is_point_inside_baseboard(Vec2 { x: p.transform.x, y: p.transform.y }, board)
            || p.connectors
                .iter()
                .any(|c| is_point_inside_baseboard(Vec2 { x: c.x, y: c.y }, board))
            || (p.transform.x >= bounds.x - 50.0
                && p.transform.x <= bounds.x + bounds.width + 50.0
                && p.transform.y >= bounds.y - 50.0
                && p.transform.y <= bounds.y + bounds.depth + 50.0);
        if near {
            qualifying.push(p);
        }
    }
    if qualifying.is_empty() {
        return Vec::new();
    }

    // Build adjacency among qualifying pieces
    let n_q = qualifying.len();
    let conn_height = |p: &TrackPiece, c_idx: usize| -> f64 {
        let base = layer_map
            .get(p.layer_id.as_deref().unwrap_or("layer-1"))
            .and_then(|l| l.height_mm)
            .unwrap_or(0.0);
        let local = p.elevation.map_or(0.0, |e| if c_idx == 0 { e.start } else { e.end });
        base + local
    };
    let mut adj: Vec<Vec<(usize, usize, usize)>> = vec![Vec::new(); n_q];
    for i in 0..n_q {
        for j in (i + 1)..n_q {
            for (ca_i, ca) in qualifying[i].connectors.iter().enumerate() {
                let ha = conn_height(qualifying[i], ca_i);
                for (cb_i, cb) in qualifying[j].connectors.iter().enumerate() {
                    let hb = conn_height(qualifying[j], cb_i);
                    if (ca.x - cb.x).hypot(ca.y - cb.y) <= 8.0 && (ha - hb).abs() <= 5.0 {
                        adj[i].push((j, ca_i, cb_i));
                        adj[j].push((i, cb_i, ca_i));
                    }
                }
            }
        }
    }

    let mut unvisited: Vec<bool> = vec![true; n_q];
    let mut cutouts = Vec::new();
    let b_min_x = bounds.x + 6.0;
    let b_max_x = bounds.x + bounds.width - 6.0;
    let b_min_y = bounds.y + 6.0;
    let b_max_y = bounds.y + bounds.depth - 6.0;

    while let Some(start) = {
        let mut best_s = None;
        let mut min_deg = usize::MAX;
        for (i, &unv) in unvisited.iter().enumerate() {
            if !unv {
                continue;
            }
            let deg = adj[i].iter().filter(|e| unvisited[e.0]).count();
            if deg < min_deg {
                min_deg = deg;
                best_s = Some(i);
                if deg <= 1 {
                    break;
                }
            }
        }
        best_s
    } {
        let mut chain: Vec<(usize, usize, usize)> = Vec::new();
        let mut cur = Some(start);
        let mut prev_idx: Option<usize> = None;

        while let Some(ci) = cur {
            unvisited[ci] = false;
            let (from_c, to_c) = if let Some(pi) = prev_idx {
                if let Some(&(_, my_c, _)) = adj[ci].iter().find(|e| e.0 == pi) {
                    (my_c, if my_c == 0 { 1 } else { 0 })
                } else {
                    (0, 1)
                }
            } else if let Some(&(_, my_c, _)) = adj[ci].iter().find(|e| unvisited[e.0]) {
                (if my_c == 0 { 1 } else { 0 }, my_c)
            } else if let Some(&(_, my_c, _)) = adj[ci].first() {
                (if my_c == 0 { 1 } else { 0 }, my_c)
            } else {
                (0, 1)
            };

            chain.push((ci, from_c, to_c));
            prev_idx = Some(ci);
            cur = adj[ci]
                .iter()
                .find(|e| e.1 == to_c && unvisited[e.0])
                .map(|e| e.0);
        }

        let mut centerline: Vec<Vec2> = Vec::new();
        let mut max_w = 0.0_f64;
        let min_cut_elev = deck_elev - clearance;
        let max_cut_elev = deck_elev;

        for (qi, from_c, to_c) in chain {
            let p = qualifying[qi];
            let entry = find_entry(catalogs, &p.catalog_id);
            let mut pw = get_track_clearance_width(entry);
            if is_road_id(catalogs, &p.catalog_id) && entry.is_none() {
                pw = 76.0;
            }
            max_w = max_w.max(pw);

            let base_h = layer_map
                .get(p.layer_id.as_deref().unwrap_or("layer-1"))
                .and_then(|l| l.height_mm)
                .unwrap_or(0.0);
            let ps = p.elevation.map_or(0.0, |e| e.start);
            let pe = p.elevation.map_or(ps, |e| e.end);
            let h0 = base_h + ps;
            let h1 = base_h + pe;
            let h_from = if from_c == 0 { h0 } else { h1 };
            let h_to = if to_c == 0 { h0 } else { h1 };
            let dh = h_to - h_from;
            let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
            if dh.abs() >= 0.5 {
                let ta = (max_cut_elev - h_from) / dh;
                let tb = (min_cut_elev - h_from) / dh;
                t0 = ta.min(tb).clamp(0.0, 1.0);
                t1 = ta.max(tb).clamp(0.0, 1.0);
            }
            if t1 <= t0 + 0.01 {
                continue;
            }
            for pt in piece_centerline_clipped(p, from_c, to_c, t0, t1) {
                if centerline
                    .last()
                    .map_or(true, |last| (pt.x - last.x).hypot(pt.y - last.y) >= 1.0)
                {
                    centerline.push(pt);
                }
            }
        }

        if centerline.len() < 2 {
            continue;
        }
        let half_w = max_w * 0.5;
        let m_len = centerline.len();
        let mut left_pts = Vec::with_capacity(m_len);
        let mut right_pts = Vec::with_capacity(m_len);

        for i in 0..m_len {
            let cur_pt = centerline[i];
            let (tx, ty) = if i == 0 {
                (centerline[1].x - cur_pt.x, centerline[1].y - cur_pt.y)
            } else if i + 1 == m_len {
                (cur_pt.x - centerline[m_len - 2].x, cur_pt.y - centerline[m_len - 2].y)
            } else {
                (
                    centerline[i + 1].x - centerline[i - 1].x,
                    centerline[i + 1].y - centerline[i - 1].y,
                )
            };
            let len = tx.hypot(ty).max(1.0);
            let nx = -ty / len;
            let ny = tx / len;
            left_pts.push(Vec2 {
                x: cur_pt.x + nx * half_w,
                y: cur_pt.y + ny * half_w,
            });
            right_pts.push(Vec2 {
                x: cur_pt.x - nx * half_w,
                y: cur_pt.y - ny * half_w,
            });
        }

        let mut raw_flat = Vec::with_capacity((left_pts.len() + right_pts.len()) * 2);
        for p in &left_pts {
            raw_flat.push(p.x);
            raw_flat.push(p.y);
        }
        for p in right_pts.iter().rev() {
            raw_flat.push(p.x);
            raw_flat.push(p.y);
        }

        let sim = simplify_cutout_polygon(&raw_flat, b_min_x, b_max_x, b_min_y, b_max_y, 3.0, 0.04);
        if sim.len() >= 6 {
            let pts: Vec<Vec2> = sim
                .chunks_exact(2)
                .map(|c| Vec2 { x: c[0], y: c[1] })
                .collect();
            if signed_polygon_area(&pts).abs() >= 10.0 {
                cutouts.push(TrackCutoutPolygon { points: pts });
            }
        }
    }

    cutouts
}

pub fn is_point_inside_cutouts(
    pt: Vec2,
    cutouts: &[TrackCutoutPolygon],
    safety_buffer: f64,
) -> bool {
    for c in cutouts {
        if c.points.len() < 3 {
            continue;
        }
        if is_point_in_polygon(pt, &c.points) {
            return true;
        }
        let n = c.points.len();
        for i in 0..n {
            if dist_to_segment(pt, c.points[i], c.points[(i + 1) % n]) <= safety_buffer {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, create_piece};
    use railbench_core::types::{Elevation, Transform};

    #[test]
    fn baseboard_shapes_generate_expected_polygons() {
        let mut l = create_default_baseboard("b1".into(), "L".into());
        l.shape = Some("l-shape".into());
        l.position_mode = "top-left".into();
        l.width = 2600.0;
        l.depth = 2000.0;
        let l_poly = get_baseboard_polygon(&l);
        assert_eq!(l_poly.len(), 6);
        assert!(is_point_inside_baseboard(Vec2 { x: 200.0, y: 200.0 }, &l));
        assert!(!is_point_inside_baseboard(Vec2 { x: 2000.0, y: 1500.0 }, &l));

        let mut u = l.clone();
        u.shape = Some("u-shape".into());
        let u_poly = get_baseboard_polygon(&u);
        assert_eq!(u_poly.len(), 8);

        let mut t = l.clone();
        t.shape = Some("t-shape".into());
        let t_poly = get_baseboard_polygon(&t);
        assert_eq!(t_poly.len(), 8);

        let mut tri = l.clone();
        tri.shape = Some("triangle".into());
        let tri_poly = get_baseboard_polygon(&tri);
        assert_eq!(tri_poly.len(), 3);
    }

    #[test]
    fn triangulate_concave_l_shape_and_polygon_with_hole() {
        let outer = vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 1000.0, y: 0.0 },
            Vec2 { x: 1000.0, y: 400.0 },
            Vec2 { x: 400.0, y: 400.0 },
            Vec2 { x: 400.0, y: 1000.0 },
            Vec2 { x: 0.0, y: 1000.0 },
        ];
        let hole = vec![
            Vec2 { x: 100.0, y: 100.0 },
            Vec2 { x: 300.0, y: 100.0 },
            Vec2 { x: 300.0, y: 300.0 },
            Vec2 { x: 100.0, y: 300.0 },
        ];
        let expected_area = signed_polygon_area(&outer).abs() - signed_polygon_area(&hole).abs();
        let tris = triangulate_polygon_with_holes(&outer, &[hole]);
        assert!(!tris.is_empty());
        let sum_area: f64 = tris
            .iter()
            .map(|tri| cross2(tri[0], tri[1], tri[2]).abs() * 0.5)
            .sum();
        assert!((sum_area - expected_area).abs() < 1.0);
    }

    #[test]
    fn snap_baseboard_aligns_edge_to_edge_and_emits_guide() {
        let mut b1 = create_default_baseboard("b1".into(), "Main".into());
        b1.position_mode = "top-left".into();
        b1.width = 2000.0;
        b1.depth = 1000.0;
        let mut b2 = create_default_baseboard("b2".into(), "Wing".into());
        b2.position_mode = "custom".into();
        b2.width = 1000.0;
        b2.depth = 800.0;

        let res = snap_baseboard(&b2, 2018.0, 12.0, &[b1], 35.0);
        assert!(res.snapped_x);
        assert!(res.snapped_y);
        assert_eq!(res.x, 2000.0);
        assert_eq!(res.y, 0.0);
        assert!(!res.guides.is_empty());
    }

    #[test]
    fn compute_table_track_cutouts_cuts_sloped_ramp_but_ignores_flat_shadow_station() {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "24360").unwrap();
        let mut ramp = create_piece(
            "r1".into(),
            entry,
            Transform { x: -180.0, y: 0.0, rotation: 0.0 },
            None,
        );
        ramp.elevation = Some(Elevation { start: 5.0, end: -45.0 });

        let mut shadow = create_piece(
            "s1".into(),
            entry,
            Transform { x: -180.0, y: 200.0, rotation: 0.0 },
            None,
        );
        shadow.elevation = Some(Elevation { start: -100.0, end: -100.0 });

        let board = create_default_baseboard("b1".into(), "Main".into());
        let layout = Layout {
            name: "CutoutTest".into(),
            scale: "H0".into(),
            pieces: vec![ramp, shadow],
            models: None,
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: Some(board.clone()),
            baseboards: Some(vec![board.clone()]),
        };

        let cutouts = compute_table_track_cutouts(&layout, &catalogs, &board, 0.0, 15.0);
        assert_eq!(cutouts.len(), 1);
        assert!(is_point_inside_cutouts(Vec2 { x: 0.0, y: 0.0 }, &cutouts, 5.0));
        assert!(!is_point_inside_cutouts(Vec2 { x: 0.0, y: 200.0 }, &cutouts, 5.0));
    }

    #[test]
    fn layer_plate_crud_and_fit_to_tracks() {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "24360").unwrap();
        let p = create_piece("p1".into(), entry, Transform { x: 100.0, y: 50.0, rotation: 0.0 }, None);
        let bb = create_default_baseboard("b1".into(), "Main".into());
        let mut seq = 2u64;
        let mut layout = Layout {
            name: "T".into(),
            scale: "H0".into(),
            pieces: vec![p],
            models: None,
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: Some(bb.clone()),
            baseboards: Some(vec![bb]),
        };

        let b2_id = add_baseboard(&mut layout, &mut seq);
        assert_eq!(layout.baseboards.as_ref().unwrap().len(), 2);
        let dup_id = duplicate_baseboard(&mut layout, &b2_id, &mut seq).unwrap();
        assert_eq!(layout.baseboards.as_ref().unwrap().len(), 3);
        assert!(remove_baseboard(&mut layout, &dup_id));
        assert_eq!(layout.baseboards.as_ref().unwrap().len(), 2);

        let mut fitted = layout.baseboards.as_ref().unwrap()[0].clone();
        assert!(fit_board_to_tracks(&layout, &mut fitted));
        assert!(is_point_inside_baseboard(Vec2 { x: 250.0, y: 50.0 }, &fitted));
    }
}
