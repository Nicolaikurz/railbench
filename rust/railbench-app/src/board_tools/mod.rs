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
                        name: Some(format!("Plate {idx}")),
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

mod geometry;
#[allow(unused_imports)]
use geometry::*;
pub use geometry::{get_baseboard_polygon, layer_plate_as_baseboard, get_layer_plate_polygon, is_point_in_polygon, is_point_inside_baseboard, is_point_near_baseboard_edge, compute_baseboard_leg_positions, signed_polygon_area, triangulate_polygon_with_holes, snap_baseboard, piece_has_catenary_overhead, get_piece_clearance_width, piece_vertical_clearance};
mod cutouts;
#[allow(unused_imports)]
use cutouts::*;
pub use cutouts::{compute_table_track_cutouts, is_point_inside_cutouts};
#[cfg(test)]
mod tests;
