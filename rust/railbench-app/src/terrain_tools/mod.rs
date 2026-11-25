use std::collections::HashMap;

use railbench_core::geometry::{
    deg2rad, rotate, sample_arc_between_connectors, sample_flex_spline_points,
};
use railbench_core::terrain::{
    mountain_noise, natural_mountain_height_at, sample_heightfield_bilinear, BIOME_AUTO,
};
use railbench_core::types::{
    BaseboardConfig, Catalog, Layout, Model3D, MountainBrushNode, TrackPiece, Transform, Vec2,
};

use crate::board_tools;
use crate::catalog::{find_entry, is_road_id};

pub const R_PORTAL: f64 = 0.82;
pub const ELLIPSE_TOLERANCE: f64 = 1.05;
pub const APPROACH_SLOPE: f64 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainBrushMode {
    SculptRaise,
    SculptLower,
    SculptSmooth,
    SculptFlatten,
    SculptGrade,
    SculptBiome,
    SculptReset,
    Paint,
    Extend,
    Stamp,
    Ground,
    Erase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainRidgeProfile {
    Uniform,
    PeakCenter,
    Varied,
}

#[derive(Debug, Clone)]
pub struct TerrainBrushState {
    pub active: bool,
    pub mode: TerrainBrushMode,
    pub radius: f64,
    pub height: f64,
    pub strength: f32,
    pub biome: u8,
    pub flatten_target_mm: Option<f32>,
    pub last_sculpt_world: Option<Vec2>,
    pub style: String,
    pub ground_type: String,
    pub ground_height: f64,
    pub ridge_profile: TerrainRidgeProfile,
    pub auto_tunnel: bool,
    pub target_mountain_id: Option<String>,
    pub is_drawing: bool,
    pub stroke_points: Vec<Vec2>,
    pub cursor_world: Option<Vec2>,
}

impl Default for TerrainBrushState {
    fn default() -> Self {
        Self {
            active: false,
            mode: TerrainBrushMode::SculptRaise,
            radius: 160.0,
            height: 220.0,
            strength: 0.55,
            biome: BIOME_AUTO,
            flatten_target_mm: None,
            last_sculpt_world: None,
            style: "alpine".into(),
            ground_type: "hill".into(),
            ground_height: 26.0,
            ridge_profile: TerrainRidgeProfile::Uniform,
            auto_tunnel: true,
            target_mountain_id: None,
            is_drawing: false,
            stroke_points: Vec::new(),
            cursor_world: None,
        }
    }
}

impl TerrainBrushState {
    pub fn is_heightfield_mode(&self) -> bool {
        matches!(
            self.mode,
            TerrainBrushMode::SculptRaise
                | TerrainBrushMode::SculptLower
                | TerrainBrushMode::SculptSmooth
                | TerrainBrushMode::SculptFlatten
                | TerrainBrushMode::SculptGrade
                | TerrainBrushMode::SculptBiome
                | TerrainBrushMode::SculptReset
        )
    }

    pub fn set_active(
        &mut self,
        active: bool,
        mode: Option<TerrainBrushMode>,
        target_mountain_id: Option<String>,
    ) {
        self.active = active;
        if let Some(m) = mode {
            self.mode = m;
        }
        if target_mountain_id.is_some() {
            self.target_mountain_id = target_mountain_id;
        }
        if !active {
            self.is_drawing = false;
            self.stroke_points.clear();
            self.cursor_world = None;
            self.flatten_target_mm = None;
            self.last_sculpt_world = None;
        }
    }

    pub fn toggle(&mut self, mode: TerrainBrushMode, target_mountain_id: Option<String>) {
        if self.active && self.mode == mode {
            self.set_active(false, None, None);
        } else {
            self.set_active(true, Some(mode), target_mountain_id);
        }
    }

    pub fn adjust_radius(&mut self, delta: f64) {
        self.radius = (self.radius + delta).round().clamp(40.0, 600.0);
    }

    pub fn adjust_strength(&mut self, delta: f32) {
        self.strength = (self.strength + delta).clamp(0.10, 1.0);
    }

    pub fn active_sculpt_op(&self, shift_smooth: bool, alt_invert: bool) -> heightfield::SculptOp {
        if shift_smooth {
            return heightfield::SculptOp::Smooth;
        }
        match self.mode {
            TerrainBrushMode::SculptRaise => {
                if alt_invert {
                    heightfield::SculptOp::Lower
                } else {
                    heightfield::SculptOp::Raise
                }
            }
            TerrainBrushMode::SculptLower => {
                if alt_invert {
                    heightfield::SculptOp::Raise
                } else {
                    heightfield::SculptOp::Lower
                }
            }
            TerrainBrushMode::SculptSmooth => heightfield::SculptOp::Smooth,
            TerrainBrushMode::SculptFlatten => {
                heightfield::SculptOp::Flatten(self.flatten_target_mm.unwrap_or(0.0))
            }
            TerrainBrushMode::SculptGrade => heightfield::SculptOp::GradeTrack,
            TerrainBrushMode::SculptBiome => heightfield::SculptOp::PaintBiome(self.biome),
            TerrainBrushMode::SculptReset => heightfield::SculptOp::Reset,
            _ => heightfield::SculptOp::Raise,
        }
    }

    /// Applies one or more interpolated brush dabs from `last_sculpt_world` to `wpos`
    /// onto `layout.terrain_field`, returning `true` when any vertex changed.
    pub fn apply_live_sculpt_step(
        &mut self,
        wpos: Vec2,
        layout: &mut Layout,
        shift_smooth: bool,
        alt_invert: bool,
    ) -> bool {
        if self.mode == TerrainBrushMode::SculptFlatten && self.flatten_target_mm.is_none() {
            let sampled = heightfield::active_layer_heightfield(layout)
                .map(|hf| sample_heightfield_bilinear(hf, wpos.x, wpos.y))
                .unwrap_or(0.0);
            self.flatten_target_mm = Some(sampled);
        }
        let op = self.active_sculpt_op(shift_smooth, alt_invert);
        let spacing = (self.radius * 0.22).max(16.0);
        let prev = self.last_sculpt_world.unwrap_or(wpos);
        let dx = wpos.x - prev.x;
        let dy = wpos.y - prev.y;
        let dist = dx.hypot(dy);
        let steps = (dist / spacing).floor().clamp(0.0, 24.0) as usize;
        let max_h = self.height as f32;
        let mut changed = false;

        for s in 1..=steps.max(1) {
            let t = if steps == 0 {
                1.0
            } else {
                s as f64 / steps as f64
            };
            let pt = Vec2 {
                x: prev.x + dx * t,
                y: prev.y + dy * t,
            };
            if heightfield::apply_heightfield_brush(
                layout,
                op,
                pt,
                self.radius,
                self.strength,
                max_h,
                self.biome,
            ) {
                changed = true;
            }
        }
        self.last_sculpt_world = Some(wpos);
        self.cursor_world = Some(wpos);
        changed
    }

    pub fn set_ground_type(&mut self, gtype: &str) {
        self.ground_type = gtype.to_string();
        self.ground_height = match gtype {
            "hill" => 26.0,
            "dirt" => 14.0,
            "sand" => 12.0,
            "water" => 6.0,
            _ => 20.0,
        };
    }

    fn compute_node_height_and_radius(
        &self,
        idx: usize,
        total: usize,
        base_r: f64,
        base_h: f64,
    ) -> (f64, f64) {
        if total <= 2 || self.ridge_profile == TerrainRidgeProfile::Uniform {
            return (base_r, base_h);
        }
        let t = idx as f64 / (total - 1) as f64;
        match self.ridge_profile {
            TerrainRidgeProfile::Uniform => (base_r, base_h),
            TerrainRidgeProfile::PeakCenter => {
                let bell = (t * std::f64::consts::PI).sin();
                (
                    (base_r * (0.82 + 0.22 * bell)).round(),
                    (base_h * (0.68 + 0.38 * bell)).round(),
                )
            }
            TerrainRidgeProfile::Varied => {
                let fi = idx as f64;
                let wave = (fi * 1.9 + 0.7).sin() * 0.16 + (fi * 3.1).cos() * 0.08;
                (
                    (base_r * (1.0 + wave * 0.5)).round(),
                    (base_h * (1.0 + wave)).round(),
                )
            }
        }
    }

    fn find_extend_target<'a>(&self, first_pt: Vec2, layout: &'a Layout) -> Option<&'a Model3D> {
        let models = layout.models.as_ref()?;
        let active_lid = layout.active_layer_id.as_deref();
        let mountains: Vec<&Model3D> = models
            .iter()
            .filter(|m| {
                m.builtin_id.as_deref() == Some("mountain")
                    && active_lid
                        .map_or(true, |lid| m.layer_id.as_deref().unwrap_or("layer-1") == lid)
            })
            .collect();
        if mountains.is_empty() {
            return None;
        }
        if let Some(ref tid) = self.target_mountain_id {
            if let Some(m) = mountains.iter().find(|m| &m.id == tid) {
                return Some(*m);
            }
        }
        mountains
            .into_iter()
            .min_by(|a, b| {
                let da = (first_pt.x - a.transform.x).hypot(first_pt.y - a.transform.y);
                let db = (first_pt.x - b.transform.x).hypot(first_pt.y - b.transform.y);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn start_stroke(&mut self, wpos: Vec2, layout: &mut Layout) -> bool {
        if !self.active {
            return false;
        }
        self.cursor_world = Some(wpos);
        self.is_drawing = true;
        if self.is_heightfield_mode() {
            self.stroke_points.clear();
            self.flatten_target_mm = None;
            self.last_sculpt_world = None;
            return self.apply_live_sculpt_step(wpos, layout, false, false);
        }
        if self.mode == TerrainBrushMode::Erase {
            self.stroke_points.clear();
            return erase_terrain_at(wpos, self.radius, layout);
        }
        let snapped = clamp_brush_pt(layout, None, Vec2 { x: wpos.x.round(), y: wpos.y.round() });
        self.stroke_points = vec![snapped];
        false
    }

    pub fn move_stroke(&mut self, wpos: Vec2, layout: &mut Layout) -> bool {
        self.cursor_world = Some(wpos);
        if !self.active || !self.is_drawing {
            return false;
        }
        if self.is_heightfield_mode() {
            return self.apply_live_sculpt_step(wpos, layout, false, false);
        }
        if self.mode == TerrainBrushMode::Erase {
            return erase_terrain_at(wpos, self.radius, layout);
        }
        let snapped = clamp_brush_pt(layout, None, Vec2 { x: wpos.x.round(), y: wpos.y.round() });
        if self.mode == TerrainBrushMode::Stamp {
            self.stroke_points = vec![snapped];
            return false;
        }
        let min_step = (self.radius * 0.22).max(24.0);
        let push = self
            .stroke_points
            .last()
            .map_or(true, |last| (snapped.x - last.x).hypot(snapped.y - last.y) >= min_step);
        if push {
            self.stroke_points.push(snapped);
        }
        false
    }

    pub fn finish_stroke(
        &mut self,
        layout: &mut Layout,
        next_seq: &mut u64,
        active_layer_id: Option<&str>,
        is_de: bool,
    ) -> Option<String> {
        if !self.is_drawing {
            return None;
        }
        self.is_drawing = false;

        if self.is_heightfield_mode() {
            self.flatten_target_mm = None;
            self.last_sculpt_world = None;
            self.stroke_points.clear();
            if self.auto_tunnel {
                heightfield::apply_auto_tunnel_for_heightfield(layout);
            }
            return None;
        }

        if self.mode == TerrainBrushMode::Erase || self.stroke_points.is_empty() {
            self.stroke_points.clear();
            return None;
        }

        let layer_id = active_layer_id
            .map(str::to_string)
            .or_else(|| layout.active_layer_id.clone())
            .or_else(|| layout.layers.as_ref().and_then(|ls| ls.first()).map(|l| l.id.clone()));

        let mut pts = if self.mode == TerrainBrushMode::Stamp {
            vec![*self.stroke_points.last().unwrap()]
        } else {
            self.stroke_points.clone()
        };
        for p in &mut pts {
            *p = clamp_brush_pt(layout, layer_id.as_deref(), *p);
        }
        self.stroke_points.clear();

        let r = self.radius.round().max(60.0);
        let h = self.height.round().max(40.0);

        if self.mode == TerrainBrushMode::Ground {
            let (mut min_x, mut max_x, mut min_y, mut max_y) =
                (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
            for p in &pts {
                min_x = min_x.min(p.x);
                max_x = max_x.max(p.x);
                min_y = min_y.min(p.y);
                max_y = max_y.max(p.y);
            }
            let cx = ((min_x + max_x) * 0.5).round();
            let cy = ((min_y + max_y) * 0.5).round();
            let g_h = self.ground_height.round().max(2.0);
            let width = if pts.len() >= 2 {
                (max_x - min_x + r * 2.15).max(r * 2.0).round()
            } else {
                r * 2.0
            };
            let depth = if pts.len() >= 2 {
                (max_y - min_y + r * 2.15).max(r * 2.0).round()
            } else {
                (r * 1.75).round()
            };
            let path_nodes = if pts.len() >= 2 {
                Some(
                    pts.iter()
                        .map(|p| MountainBrushNode {
                            x: (p.x - cx).round(),
                            y: (p.y - cy).round(),
                            radius: r,
                            height: g_h,
                        })
                        .collect(),
                )
            } else {
                None
            };

            let models = layout.models.get_or_insert_with(Vec::new);
            let existing = models
                .iter()
                .filter(|m| m.builtin_id.as_deref() == Some("ground-terrain"))
                .count();
            let type_label = match (self.ground_type.as_str(), is_de) {
                ("water", true) => "Wasser",
                ("water", false) => "Water",
                ("sand", _) => "Sand",
                ("dirt", true) => "Erde",
                ("dirt", false) => "Dirt",
                (_, true) => "Huegel",
                (_, false) => "Hill",
            };
            let id = format!("terrain-{}", *next_seq);
            *next_seq += 1;
            models.push(Model3D {
                id: id.clone(),
                name: format!("{type_label} {}", existing + 1),
                category: "terrain".into(),
                transform: Transform { x: cx, y: cy, rotation: 0.0 },
                elevation: Some(0.0),
                scale: Some(1.0),
                layer_id,
                width,
                depth,
                height: g_h,
                model_type: "builtin".into(),
                builtin_id: Some("ground-terrain".into()),
                mountain_style: None,
                mountain_path: path_nodes,
                ground_terrain_type: Some(self.ground_type.clone()),
                file_data: None,
                file_name: None,
            });
            return Some(id);
        }

        if self.mode == TerrainBrushMode::Extend {
            let target_id = self.find_extend_target(pts[0], layout).map(|m| m.id.clone());
            if let Some(tid) = target_id {
                let auto_tunnel = self.auto_tunnel;
                let mut world_nodes: Vec<MountainBrushNode> = Vec::new();
                if let Some(m) = layout
                    .models
                    .as_ref()
                    .and_then(|ms| ms.iter().find(|m| m.id == tid))
                {
                    if let Some(ref path) = m.mountain_path {
                        if !path.is_empty() {
                            for n in path {
                                let (wx, wy) = mountain_local_to_world(n.x, n.y, Some(m));
                                world_nodes.push(MountainBrushNode {
                                    x: wx,
                                    y: wy,
                                    radius: n.radius,
                                    height: n.height,
                                });
                            }
                        }
                    }
                    if world_nodes.is_empty() {
                        world_nodes.push(MountainBrushNode {
                            x: m.transform.x,
                            y: m.transform.y,
                            radius: (m.width.max(m.depth) * 0.45).round().max(60.0),
                            height: m.height.max(40.0),
                        });
                    }
                }

                for (idx, p) in pts.iter().enumerate() {
                    let (nr, nh) = self.compute_node_height_and_radius(idx, pts.len(), r, h);
                    world_nodes.push(MountainBrushNode {
                        x: p.x.round(),
                        y: p.y.round(),
                        radius: nr,
                        height: nh,
                    });
                }

                if let Some(models) = &mut layout.models {
                    if let Some(m) = models.iter_mut().find(|m| m.id == tid) {
                        recompute_mountain_bounds_from_world_nodes(m, &world_nodes);
                    }
                }
                if auto_tunnel {
                    if let Some(m_clone) = layout
                        .models
                        .as_ref()
                        .and_then(|ms| ms.iter().find(|m| m.id == tid).cloned())
                    {
                        apply_auto_tunnel_for_mountain(&m_clone, layout);
                    }
                }
                self.target_mountain_id = Some(tid.clone());
                return Some(tid);
            }
        }

        // Paint or Stamp new mountain
        let (mut min_x, mut max_x, mut min_y, mut max_y) =
            (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for p in &pts {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        let cx = ((min_x + max_x) * 0.5).round();
        let cy = ((min_y + max_y) * 0.5).round();

        let (width, depth, max_node_h, mountain_path) = if pts.len() >= 2 {
            let mut max_nh = h;
            let mut nodes = Vec::with_capacity(pts.len());
            for (idx, p) in pts.iter().enumerate() {
                let (nr, nh) = self.compute_node_height_and_radius(idx, pts.len(), r, h);
                max_nh = max_nh.max(nh);
                nodes.push(MountainBrushNode {
                    x: (p.x - cx).round(),
                    y: (p.y - cy).round(),
                    radius: nr,
                    height: nh,
                });
            }
            (
                (max_x - min_x + r * 2.15).max(r * 2.0).round(),
                (max_y - min_y + r * 2.15).max(r * 2.0).round(),
                max_nh,
                Some(nodes),
            )
        } else {
            (r * 2.0, (r * 1.75).round(), h, None)
        };

        let existing_cnt = layout
            .models
            .as_ref()
            .map_or(0, |ms| ms.iter().filter(|m| m.builtin_id.as_deref() == Some("mountain")).count());
        let name = if pts.len() >= 2 {
            if is_de {
                format!("Bergkette {}", existing_cnt + 1)
            } else {
                format!("Mountain Ridge {}", existing_cnt + 1)
            }
        } else if is_de {
            format!("Berg {}", existing_cnt + 1)
        } else {
            format!("Mountain {}", existing_cnt + 1)
        };

        let id = format!("mtn-{}", *next_seq);
        *next_seq += 1;
        let new_mtn = Model3D {
            id: id.clone(),
            name,
            category: "terrain".into(),
            transform: Transform { x: cx, y: cy, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id,
            width,
            depth,
            height: max_node_h,
            model_type: "builtin".into(),
            builtin_id: Some("mountain".into()),
            mountain_style: Some(self.style.clone()),
            mountain_path,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };
        layout.models.get_or_insert_with(Vec::new).push(new_mtn.clone());
        if self.auto_tunnel {
            apply_auto_tunnel_for_mountain(&new_mtn, layout);
        }
        self.target_mountain_id = Some(id.clone());
        Some(id)
    }
}

fn clamp_brush_pt(layout: &Layout, layer_id: Option<&str>, pt: Vec2) -> Vec2 {
    let eff_lid = layer_id
        .map(str::to_string)
        .or_else(|| layout.active_layer_id.clone())
        .or_else(|| layout.layers.as_ref().and_then(|ls| ls.first()).map(|l| l.id.clone()));
    let boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
        .into_iter()
        .filter(|b| b.enabled)
        .collect();
    let polys = collect_terrain_support_polygons(layout, eff_lid.as_deref(), &boards);
    if polys.is_empty() {
        return pt;
    }
    let (offsets, coords) = pack_support_polygons(&polys);
    let c = clamp_point_to_support_polygons(pt, &polys, &offsets, &coords);
    Vec2 {
        x: c.x.round(),
        y: c.y.round(),
    }
}

#[inline]
pub fn world_to_mountain_local(wx: f64, wy: f64, mountain: &Model3D) -> (f64, f64) {
    let rad = deg2rad(-mountain.transform.rotation);
    let cos_r = rad.cos();
    let sin_r = rad.sin();
    let dx = wx - mountain.transform.x;
    let dy = wy - mountain.transform.y;
    (dx * cos_r - dy * sin_r, dx * sin_r + dy * cos_r)
}

#[inline]
pub fn mountain_local_to_world(lx: f64, lz: f64, mountain: Option<&Model3D>) -> (f64, f64) {
    let Some(m) = mountain else {
        return (lx, lz);
    };
    let rad = deg2rad(m.transform.rotation);
    let cos_f = rad.cos();
    let sin_f = rad.sin();
    (
        m.transform.x + (lx * cos_f - lz * sin_f),
        m.transform.y + (lx * sin_f + lz * cos_f),
    )
}

pub fn make_mountain_preset_model(
    preset: &str,
    id: String,
    existing_count: usize,
    style: &str,
    bounds: board_tools::BoardBounds,
    layer_id: Option<String>,
    is_de: bool,
) -> Model3D {
    let (name, cx, cy, w, d, h, path) = match preset {
        "corner" => {
            let cx = (bounds.x + (bounds.width * 0.25).min(420.0)).round();
            let cy = (bounds.y + (bounds.depth * 0.28).min(340.0)).round();
            let n = if is_de {
                format!("Eck-Gebirge {}", existing_count + 1)
            } else {
                format!("Corner Massif {}", existing_count + 1)
            };
            (
                n,
                cx,
                cy,
                960.0,
                780.0,
                240.0,
                Some(vec![
                    MountainBrushNode { x: -200.0, y: 180.0, radius: 220.0, height: 210.0 },
                    MountainBrushNode { x: -200.0, y: -140.0, radius: 250.0, height: 240.0 },
                    MountainBrushNode { x: 220.0, y: -160.0, radius: 220.0, height: 220.0 },
                ]),
            )
        }
        "ridge" => {
            let cx = (bounds.x + bounds.width * 0.5).round();
            let cy = (bounds.y + (bounds.depth * 0.18).min(200.0)).round();
            let rw = (bounds.width * 0.72).max(800.0).round();
            let half_span = (rw * 0.34).round();
            let n = if is_de {
                format!("Gebirgskette {}", existing_count + 1)
            } else {
                format!("Backdrop Ridge {}", existing_count + 1)
            };
            (
                n,
                cx,
                cy,
                rw,
                420.0,
                250.0,
                Some(vec![
                    MountainBrushNode { x: -half_span, y: 15.0, radius: 190.0, height: 210.0 },
                    MountainBrushNode { x: 0.0, y: -15.0, radius: 210.0, height: 250.0 },
                    MountainBrushNode { x: half_span, y: 10.0, radius: 190.0, height: 220.0 },
                ]),
            )
        }
        _ => {
            let off = ((existing_count % 5) as f64) * 60.0;
            let cx = (bounds.x + bounds.width * 0.5 + off).round();
            let cy = (bounds.y + bounds.depth * 0.5 + off).round();
            let n = if is_de {
                format!("Berg {}", existing_count + 1)
            } else {
                format!("Mountain {}", existing_count + 1)
            };
            (n, cx, cy, 600.0, 450.0, 220.0, None)
        }
    };

    Model3D {
        id,
        name,
        category: "terrain".into(),
        transform: Transform { x: cx, y: cy, rotation: 0.0 },
        elevation: Some(0.0),
        scale: Some(1.0),
        layer_id,
        width: w,
        depth: d,
        height: h,
        model_type: "builtin".into(),
        builtin_id: Some("mountain".into()),
        mountain_style: Some(style.to_string()),
        mountain_path: path,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    }
}

mod mountain_ops;
#[allow(unused_imports)]
use mountain_ops::*;
#[allow(unused_imports)]
pub use mountain_ops::{
    apply_auto_tunnel_all_mountains, apply_auto_tunnel_for_mountain,
    clamp_point_to_support_polygons, clip_segment_to_support, collect_terrain_support_polygons,
    erase_terrain_at, erase_terrain_at_on_layer, evaluate_mountain_local,
    is_point_inside_any_mountain, is_point_inside_other_mountain, pack_support_polygons,
    recompute_mountain_bounds_from_world_nodes, smooth_mountain_path, TunnelProfile,
};
mod corridors;
#[allow(unused_imports)]
use corridors::*;
#[allow(unused_imports)]
pub use corridors::{create_tunnel_profile, tunnel_profile_points, CorridorChain, build_connected_corridor_chains, LocalTunnelPt, TunnelPortalInfo, TunnelRun, compute_tunnel_runs_for_mountain, count_mountain_tunnel_collisions};
mod heightfield;
pub use heightfield::*;
#[cfg(test)]
mod tests;
