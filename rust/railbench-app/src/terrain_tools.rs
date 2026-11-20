use std::collections::HashMap;

use railbench_core::geometry::{
    deg2rad, rotate, sample_arc_between_connectors, sample_flex_spline_points,
};
use railbench_core::terrain::{mountain_noise, natural_mountain_height_at};
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
            mode: TerrainBrushMode::Paint,
            radius: 220.0,
            height: 220.0,
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
        self.radius = (self.radius + delta).round().clamp(60.0, 600.0);
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
        let mountains: Vec<&Model3D> = models
            .iter()
            .filter(|m| m.builtin_id.as_deref() == Some("mountain"))
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
        if self.mode == TerrainBrushMode::Erase {
            self.stroke_points.clear();
            return erase_terrain_at(wpos, self.radius, layout);
        }
        let snapped = Vec2 {
            x: wpos.x.round(),
            y: wpos.y.round(),
        };
        self.stroke_points = vec![snapped];
        false
    }

    pub fn move_stroke(&mut self, wpos: Vec2, layout: &mut Layout) -> bool {
        self.cursor_world = Some(wpos);
        if !self.active || !self.is_drawing {
            return false;
        }
        if self.mode == TerrainBrushMode::Erase {
            return erase_terrain_at(wpos, self.radius, layout);
        }
        let snapped = Vec2 {
            x: wpos.x.round(),
            y: wpos.y.round(),
        };
        if self.mode == TerrainBrushMode::Stamp {
            self.stroke_points = vec![snapped];
            return false;
        }
        let min_step = (self.radius * 0.22).max(24.0);
        let push = self
            .stroke_points
            .last()
            .map_or(true, |last| (wpos.x - last.x).hypot(wpos.y - last.y) >= min_step);
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

        if self.mode == TerrainBrushMode::Erase || self.stroke_points.is_empty() {
            self.stroke_points.clear();
            return None;
        }

        let pts = if self.mode == TerrainBrushMode::Stamp {
            vec![*self.stroke_points.last().unwrap()]
        } else {
            self.stroke_points.clone()
        };
        self.stroke_points.clear();

        let r = self.radius.round().max(60.0);
        let h = self.height.round().max(40.0);
        let layer_id = active_layer_id
            .map(str::to_string)
            .or_else(|| layout.active_layer_id.clone())
            .or_else(|| layout.layers.as_ref().and_then(|ls| ls.first()).map(|l| l.id.clone()));

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

/// Evaluates normalized boundary distance `norm_r` (`0` = summit/spine, `1` = toe)
/// and natural surface height `base_h` in local unscaled-rotated coordinates of `mountain`.
/// Respects `m.scale` so scaled models match 2D and 3D bounds identically.
pub fn evaluate_mountain_local(lx: f64, lz: f64, mountain: &Model3D) -> (f64, f64) {
    let sc = mountain.scale.unwrap_or(1.0).max(0.05);
    let ux = lx / sc;
    let uz = lz / sc;
    let height = mountain.height.max(1.0);
    let half_w = (mountain.width * 0.5).max(1.0);
    let half_d = (mountain.depth * 0.5).max(1.0);

    if let Some(ref path) = mountain.mountain_path {
        if !path.is_empty() {
            let noise_val = mountain_noise(ux / 220.0, uz / 220.0);
            if path.len() == 1 {
                let n = &path[0];
                let r = n.radius.max(25.0);
                let h = if n.height > 0.0 { n.height } else { height };
                let d = (ux - n.x).hypot(uz - n.y);
                let norm_r = d / r;
                let base_h = if norm_r < 1.0 {
                    let prof = (norm_r * std::f64::consts::FRAC_PI_2).cos().powf(1.35);
                    (h * prof * (1.0 + 0.26 * noise_val)).max(0.0)
                } else {
                    0.0
                };
                return (norm_r, base_h * sc);
            }

            let mut min_norm_r = f64::INFINITY;
            let mut max_base_h = 0.0_f64;
            for w in path.windows(2) {
                let a = &w[0];
                let b = &w[1];
                let abx = b.x - a.x;
                let abz = b.y - a.y;
                let len_sq = abx * abx + abz * abz;
                let t = if len_sq > 1e-4 {
                    (((ux - a.x) * abx + (uz - a.y) * abz) / len_sq).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let px = a.x + abx * t;
                let pz = a.y + abz * t;
                let ar = if a.radius > 0.0 { a.radius } else { 180.0 };
                let br = if b.radius > 0.0 { b.radius } else { 180.0 };
                let ah = if a.height > 0.0 { a.height } else { height };
                let bh = if b.height > 0.0 { b.height } else { height };
                let r = (ar + (br - ar) * t).max(25.0);
                let h = (ah + (bh - ah) * t).max(10.0);
                let d = (ux - px).hypot(uz - pz);
                let norm_r = d / r;
                if norm_r < min_norm_r {
                    min_norm_r = norm_r;
                }
                if norm_r < 1.0 {
                    let prof = (norm_r * std::f64::consts::FRAC_PI_2).cos().powf(1.35);
                    let seg_h = (h * prof * (1.0 + 0.26 * noise_val)).max(0.0);
                    if seg_h > max_base_h {
                        max_base_h = seg_h;
                    }
                }
            }
            return (min_norm_r, max_base_h * sc);
        }
    }

    let nx = ux / half_w;
    let nz = uz / half_d;
    let norm_r = nx.hypot(nz);
    let base_h = natural_mountain_height_at(nx, nz, height);
    (norm_r, base_h * sc)
}

pub fn recompute_mountain_bounds_from_world_nodes(
    mountain: &mut Model3D,
    world_nodes: &[MountainBrushNode],
) {
    if world_nodes.is_empty() {
        return;
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) =
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    let mut max_r = 60.0_f64;
    let mut max_h = 40.0_f64;

    for n in world_nodes {
        min_x = min_x.min(n.x - n.radius);
        max_x = max_x.max(n.x + n.radius);
        min_y = min_y.min(n.y - n.radius);
        max_y = max_y.max(n.y + n.radius);
        max_r = max_r.max(n.radius);
        max_h = max_h.max(n.height);
    }

    let cx = ((min_x + max_x) * 0.5).round();
    let cy = ((min_y + max_y) * 0.5).round();
    mountain.transform.x = cx;
    mountain.transform.y = cy;
    mountain.transform.rotation = 0.0;
    mountain.height = max_h.round();

    if world_nodes.len() == 1 {
        let only = &world_nodes[0];
        mountain.width = (only.radius * 2.0).round();
        mountain.depth = (only.radius * 1.75).round();
        mountain.mountain_path = None;
    } else {
        mountain.width = (max_x - min_x + max_r * 0.25).max(max_r * 2.0).round();
        mountain.depth = (max_y - min_y + max_r * 0.25).max(max_r * 2.0).round();
        mountain.mountain_path = Some(
            world_nodes
                .iter()
                .map(|n| MountainBrushNode {
                    x: (n.x - cx).round(),
                    y: (n.y - cy).round(),
                    radius: n.radius.round(),
                    height: n.height.round(),
                })
                .collect(),
        );
    }
}

pub fn smooth_mountain_path(mountain: &mut Model3D) -> bool {
    let Some(ref path) = mountain.mountain_path else {
        return false;
    };
    if path.len() < 3 {
        return false;
    }
    let world_nodes: Vec<MountainBrushNode> = path
        .iter()
        .map(|n| {
            let (wx, wy) = mountain_local_to_world(n.x, n.y, Some(mountain));
            MountainBrushNode {
                x: wx,
                y: wy,
                radius: n.radius,
                height: n.height,
            }
        })
        .collect();

    let mut smoothed = Vec::with_capacity(world_nodes.len());
    for i in 0..world_nodes.len() {
        if i == 0 || i + 1 == world_nodes.len() {
            smoothed.push(world_nodes[i]);
        } else {
            let prev = &world_nodes[i - 1];
            let curr = &world_nodes[i];
            let next = &world_nodes[i + 1];
            smoothed.push(MountainBrushNode {
                x: (prev.x * 0.25 + curr.x * 0.5 + next.x * 0.25).round(),
                y: (prev.y * 0.25 + curr.y * 0.5 + next.y * 0.25).round(),
                radius: (prev.radius * 0.2 + curr.radius * 0.6 + next.radius * 0.2).round(),
                height: (prev.height * 0.2 + curr.height * 0.6 + next.height * 0.2).round(),
            });
        }
    }

    recompute_mountain_bounds_from_world_nodes(mountain, &smoothed);
    true
}

fn is_point_inside_terrain_model(wp: Vec2, m: &Model3D) -> bool {
    let sc = m.scale.unwrap_or(1.0).max(0.05);
    let (lx, lz) = world_to_mountain_local(wp.x, wp.y, m);
    if let Some(ref nodes) = m.mountain_path {
        if !nodes.is_empty() {
            let (nr, _) = evaluate_mountain_local(lx, lz, m);
            if nr <= 1.0 {
                return true;
            }
        }
    }
    let hw = (m.width * sc * 0.5).max(15.0);
    let hd = (m.depth * sc * 0.5).max(15.0);
    lx.abs() <= hw && lz.abs() <= hd
}

/// Node-accurate & whole-model eraser for `mountain` and `ground-terrain` models.
pub fn erase_terrain_at(wpos: Vec2, brush_radius: f64, layout: &mut Layout) -> bool {
    let Some(models) = layout.models.take() else {
        return false;
    };
    let mut changed = false;
    let erase_hit_r = (brush_radius * 0.6).max(40.0);
    let mut remaining = Vec::with_capacity(models.len());

    for mut m in models {
        if !matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain")) {
            remaining.push(m);
            continue;
        }

        if let Some(ref path) = m.mountain_path {
            if path.len() >= 2 {
                let world_nodes: Vec<MountainBrushNode> = path
                    .iter()
                    .map(|n| {
                        let (wx, wy) = mountain_local_to_world(n.x, n.y, Some(&m));
                        MountainBrushNode {
                            x: wx,
                            y: wy,
                            radius: n.radius,
                            height: n.height,
                        }
                    })
                    .collect();
                let kept: Vec<MountainBrushNode> = world_nodes
                    .iter()
                    .copied()
                    .filter(|n| {
                        (wpos.x - n.x).hypot(wpos.y - n.y) > erase_hit_r.max(n.radius * 0.45)
                    })
                    .collect();
                if kept.len() != world_nodes.len() {
                    changed = true;
                    if !kept.is_empty() {
                        recompute_mountain_bounds_from_world_nodes(&mut m, &kept);
                        remaining.push(m);
                    }
                    continue;
                }
            }
        }

        if is_point_inside_terrain_model(wpos, &m)
            || (wpos.x - m.transform.x).hypot(wpos.y - m.transform.y) <= erase_hit_r
        {
            changed = true;
            continue;
        }

        remaining.push(m);
    }

    layout.models = Some(remaining);
    changed
}

fn layer_height_visible_map(layout: &Layout) -> HashMap<&str, (f64, bool)> {
    let mut map = HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            map.insert(l.id.as_str(), (l.height_mm.unwrap_or(0.0), l.visible));
        }
    }
    map
}

pub fn apply_auto_tunnel_for_mountain(mountain: &Model3D, layout: &mut Layout) -> usize {
    let m_height = mountain.height.max(20.0) * mountain.scale.unwrap_or(1.0);
    let layer_info: HashMap<String, (f64, bool)> = layout
        .layers
        .as_ref()
        .map(|ls| {
            ls.iter()
                .map(|l| (l.id.clone(), (l.height_mm.unwrap_or(0.0), l.visible)))
                .collect()
        })
        .unwrap_or_default();

    let m_layer_h = mountain
        .layer_id
        .as_deref()
        .and_then(|lid| layer_info.get(lid))
        .map(|(h, _)| *h)
        .unwrap_or(0.0);
    let m_base_elev = m_layer_h + mountain.elevation.unwrap_or(0.0);

    let mut count = 0usize;
    for piece in &mut layout.pieces {
        let (p_base_h, vis) = piece
            .layer_id
            .as_deref()
            .and_then(|lid| layer_info.get(lid))
            .copied()
            .unwrap_or((0.0, true));
        if !vis {
            continue;
        }
        let s_rel = p_base_h + piece.elevation.map(|e| e.start).unwrap_or(0.0) - m_base_elev;
        let e_rel = p_base_h
            + piece
                .elevation
                .map(|e| e.end)
                .unwrap_or_else(|| piece.elevation.map(|e| e.start).unwrap_or(0.0))
            - m_base_elev;
        if s_rel.max(e_rel) < -35.0 || s_rel.min(e_rel) > m_height - 10.0 {
            continue;
        }

        let mut test_pts = vec![Vec2 {
            x: piece.transform.x,
            y: piece.transform.y,
        }];
        for c in &piece.connectors {
            test_pts.push(Vec2 { x: c.x, y: c.y });
        }
        if piece.connectors.len() >= 2 {
            let c0 = &piece.connectors[0];
            let c1 = &piece.connectors[1];
            test_pts.push(Vec2 {
                x: (c0.x + c1.x) * 0.5,
                y: (c0.y + c1.y) * 0.5,
            });
            test_pts.push(Vec2 {
                x: c0.x * 0.75 + c1.x * 0.25,
                y: c0.y * 0.75 + c1.y * 0.25,
            });
            test_pts.push(Vec2 {
                x: c0.x * 0.25 + c1.x * 0.75,
                y: c0.y * 0.25 + c1.y * 0.75,
            });
        }

        let inside = test_pts.iter().any(|pt| {
            let (lx, lz) = world_to_mountain_local(pt.x, pt.y, mountain);
            let (nr, _) = evaluate_mountain_local(lx, lz, mountain);
            nr <= 0.85
        });

        if inside {
            piece.has_tunnel = Some(true);
            piece.has_bridge = Some(false);
            count += 1;
        }
    }
    count
}

pub fn apply_auto_tunnel_all_mountains(layout: &mut Layout) -> usize {
    let mountains: Vec<Model3D> = layout
        .models
        .as_ref()
        .map(|ms| {
            ms.iter()
                .filter(|m| m.builtin_id.as_deref() == Some("mountain"))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    let mut total = 0usize;
    for m in &mountains {
        total += apply_auto_tunnel_for_mountain(m, layout);
    }
    total
}

/// Checks if a world point lies inside `other` mountain's interior (`norm_r <= R_PORTAL - 0.04`).
pub fn is_point_inside_other_mountain(
    wx: f64,
    wy: f64,
    world_elev: f64,
    current_id: &str,
    all_mountains: &[&Model3D],
    layer_info: &HashMap<&str, (f64, bool)>,
) -> bool {
    for other in all_mountains {
        if other.id == current_id || other.builtin_id.as_deref() != Some("mountain") {
            continue;
        }
        let m_lid = other.layer_id.as_deref().unwrap_or("layer-1");
        let (mh_layer, vis) = layer_info.get(m_lid).copied().unwrap_or((0.0, true));
        if !vis {
            continue;
        }
        let m_base = mh_layer + other.elevation.unwrap_or(0.0);
        let rel_elev = world_elev - m_base;
        let other_h = other.height.max(20.0) * other.scale.unwrap_or(1.0);
        if rel_elev < -35.0 || rel_elev > other_h - 10.0 {
            continue;
        }
        let (lx, lz) = world_to_mountain_local(wx, wy, other);
        let (norm_r, _) = evaluate_mountain_local(lx, lz, other);
        if norm_r <= R_PORTAL - 0.04 {
            return true;
        }
    }
    false
}

/// Checks if a world point lies inside any visible mountain (`norm_r <= max_norm_r`).
pub fn is_point_inside_any_mountain(
    wx: f64,
    wy: f64,
    world_elev: f64,
    layout: &Layout,
    layer_info: &HashMap<&str, (f64, bool)>,
    max_norm_r: f64,
) -> bool {
    let Some(models) = &layout.models else {
        return false;
    };
    for m in models {
        if m.builtin_id.as_deref() != Some("mountain") {
            continue;
        }
        let lid = m.layer_id.as_deref().unwrap_or("layer-1");
        let (lh, vis) = layer_info.get(lid).copied().unwrap_or((0.0, true));
        if !vis {
            continue;
        }
        let m_base = lh + m.elevation.unwrap_or(0.0);
        let rel_elev = world_elev - m_base;
        let mh = m.height.max(20.0) * m.scale.unwrap_or(1.0);
        if rel_elev < -35.0 || rel_elev > mh - 10.0 {
            continue;
        }
        let (lx, lz) = world_to_mountain_local(wx, wy, m);
        let (norm_r, _) = evaluate_mountain_local(lx, lz, m);
        if norm_r <= max_norm_r {
            return true;
        }
    }
    false
}

/// Collects active table / layer sub-plate polygons for clipping 3D terrain so no overhang escapes baseboard edges.
pub fn collect_terrain_support_polygons(
    layout: &Layout,
    layer_id: Option<&str>,
    boards: &[BaseboardConfig],
) -> Vec<Vec<Vec2>> {
    let active_boards: Vec<&BaseboardConfig> = boards.iter().filter(|b| b.enabled).collect();
    let layer = layer_id
        .and_then(|lid| {
            layout
                .layers
                .as_ref()
                .and_then(|ls| ls.iter().find(|l| l.id == lid))
        })
        .or_else(|| {
            layout.active_layer_id.as_deref().and_then(|aid| {
                layout
                    .layers
                    .as_ref()
                    .and_then(|ls| ls.iter().find(|l| l.id == aid))
            })
        })
        .or_else(|| layout.layers.as_ref().and_then(|ls| ls.first()));

    let mut polys: Vec<Vec<Vec2>> = Vec::new();
    if let Some(lay) = layer {
        if let Some(plates) = &lay.plates {
            for plate in plates {
                let target_board = plate
                    .baseboard_id
                    .as_deref()
                    .and_then(|bid| {
                        active_boards
                            .iter()
                            .copied()
                            .find(|b| b.id.as_deref() == Some(bid))
                    })
                    .or_else(|| active_boards.first().copied());
                let poly = board_tools::get_layer_plate_polygon(plate, target_board);
                if poly.len() >= 3 {
                    polys.push(poly);
                }
            }
        }
    }

    let need_tables = polys.is_empty()
        || layer.map_or(true, |l| {
            l.height_mm.unwrap_or(0.0).abs() < 1e-6 || l.wood_plate_mode.as_deref() == Some("table")
        });
    if need_tables {
        for bb in active_boards {
            let poly = board_tools::get_baseboard_polygon(bb);
            if poly.len() >= 3 {
                polys.push(poly);
            }
        }
    }

    polys
}

#[derive(Debug, Clone, Copy)]
pub struct TunnelProfile {
    pub is_road: bool,
    pub has_catenary: bool,
    pub num_tracks: usize,
    pub inner_width: f32,
    pub wall_h: f32,
    pub arch_rise: f32,
    pub pier: f32,
    pub total_h: f32,
}

pub fn create_tunnel_profile(
    is_road: bool,
    has_catenary: bool,
    gauge: f64,
    num_tracks: usize,
    track_spacing: f64,
) -> TunnelProfile {
    if is_road {
        return TunnelProfile {
            is_road: true,
            has_catenary: false,
            num_tracks: 1,
            inner_width: 80.0,
            wall_h: 52.0,
            arch_rise: 20.0,
            pier: 14.0,
            total_h: 72.0,
        };
    }

    let scale = (gauge / 16.5).clamp(0.4, 3.0) as f32;
    let base_single_w = (if has_catenary { 70.0 } else { 60.0 }) * scale;
    let base_wall_h = (if has_catenary { 72.0 } else { 56.0 }) * scale;
    let base_arch_rise = (if has_catenary { 32.0 } else { 28.0 }) * scale;
    let pier = (16.0 * scale).round();

    if num_tracks <= 1 {
        let w = base_single_w.round();
        let wh = base_wall_h.round();
        let ar = base_arch_rise.round();
        return TunnelProfile {
            is_road: false,
            has_catenary,
            num_tracks: 1,
            inner_width: w,
            wall_h: wh,
            arch_rise: ar,
            pier,
            total_h: wh + ar,
        };
    }

    let total_spread = (track_spacing as f32).max(((num_tracks - 1) as f32) * 58.0 * scale);
    let multi_w = (total_spread + base_single_w).round();
    let multi_wh = ((if has_catenary { 74.0 } else { 58.0 }) * scale).round();
    let multi_ar = ((if has_catenary { 38.0 } else { 32.0 }) * scale).round();
    TunnelProfile {
        is_road: false,
        has_catenary,
        num_tracks,
        inner_width: multi_w,
        wall_h: multi_wh,
        arch_rise: multi_ar,
        pier: (pier * 1.15).round(),
        total_h: multi_wh + multi_ar,
    }
}

/// Cross-section points from bottom-left over the circular arch to bottom-right.
pub fn tunnel_profile_points(prof: &TunnelProfile, inset: f32, arc_steps: usize) -> Vec<[f32; 2]> {
    let steps = arc_steps.max(4);
    let c = (prof.inner_width * 0.5 - inset).max(6.0);
    let rise = (prof.arch_rise - inset).max(2.0);
    let wall_h = prof.wall_h.max(4.0);

    let r_circle = (c * c + rise * rise) / (2.0 * rise);
    let cy = wall_h + rise - r_circle;
    let th_l = (wall_h - cy).atan2(-c);
    let th_r = (wall_h - cy).atan2(c);

    let mut pts = Vec::with_capacity(steps + 3);
    pts.push([-c, 0.0]);
    pts.push([-c, wall_h]);
    for i in 1..steps {
        let th = th_l + (th_r - th_l) * (i as f32 / steps as f32);
        pts.push([r_circle * th.cos(), cy + r_circle * th.sin()]);
    }
    pts.push([c, wall_h]);
    pts.push([c, 0.0]);
    pts
}

#[derive(Debug, Clone)]
pub struct CorridorChain {
    pub pts_xze: Vec<[f32; 3]>, // [world_x, world_z, world_elev]
    pub half_w: f32,
    pub is_road: bool,
    pub has_catenary: bool,
    pub gauge: f64,
    pub is_turntable: bool,
    pub start_connected: bool,
    pub end_connected: bool,
}

fn piece_world_elev(piece: &TrackPiece, layer_h: f64, conn_idx: usize) -> f64 {
    let ev = piece.elevation.unwrap_or_default();
    if conn_idx == 0 {
        layer_h + ev.start
    } else {
        layer_h + ev.end
    }
}

fn sample_two_conn_segment_xze(
    piece: &TrackPiece,
    entry_c: usize,
    exit_c: usize,
    layer_h: f64,
) -> Vec<[f32; 3]> {
    let e_in = piece_world_elev(piece, layer_h, entry_c);
    let e_out = piece_world_elev(piece, layer_h, exit_c);

    if let (Some(offsets), Some(chord)) = (&piece.custom_offsets, piece.custom_chord_dist) {
        if offsets.len() >= 3 {
            let steps = 20usize;
            let flat = sample_flex_spline_points(chord, offsets[0], offsets[1], offsets[2], steps);
            let mut raw = Vec::with_capacity(steps + 1);
            for k in 0..=steps {
                let u = k as f64 / steps as f64;
                let rp = rotate(Vec2 { x: flat[k * 5], y: flat[k * 5 + 1] }, piece.transform.rotation);
                let e_pt = if entry_c == 0 {
                    e_in + (e_out - e_in) * u
                } else {
                    e_out + (e_in - e_out) * u
                };
                raw.push([
                    (piece.transform.x + rp.x) as f32,
                    (piece.transform.y + rp.y) as f32,
                    e_pt as f32,
                ]);
            }
            if entry_c == 1 {
                raw.reverse();
            }
            return raw;
        }
    }

    let c = &piece.connectors;
    if entry_c >= c.len() || exit_c >= c.len() {
        return Vec::new();
    }
    let chord = (c[exit_c].x - c[entry_c].x).hypot(c[exit_c].y - c[entry_c].y);
    let steps = ((chord / 14.0).ceil() as usize).clamp(4, 48);
    let pts2d = sample_arc_between_connectors(
        c[entry_c].x,
        c[entry_c].y,
        c[entry_c].angle,
        c[exit_c].x,
        c[exit_c].y,
        c[exit_c].angle,
        steps,
    );
    let n = pts2d.len().saturating_sub(1).max(1);
    pts2d
        .into_iter()
        .enumerate()
        .map(|(k, p)| {
            let u = k as f64 / n as f64;
            [p.x as f32, p.y as f32, (e_in + (e_out - e_in) * u) as f32]
        })
        .collect()
}

fn push_dense_subdivided(out: &mut Vec<[f32; 3]>, raw: &[[f32; 3]]) {
    for (si, w) in raw.windows(2).enumerate() {
        let a = w[0];
        let b = w[1];
        let seg_len = (b[0] - a[0]).hypot(b[1] - a[1]);
        let sub = ((seg_len / 14.0).ceil() as usize).max(1);
        let start_k = if si == 0 && out.is_empty() { 0 } else { 1 };
        for k in start_k..=sub {
            let u = k as f32 / sub as f32;
            let cand = [
                a[0] + (b[0] - a[0]) * u,
                a[1] + (b[1] - a[1]) * u,
                a[2] + (b[2] - a[2]) * u,
            ];
            if let Some(last) = out.last() {
                if (cand[0] - last[0]).hypot(cand[1] - last[1]) < 0.8 {
                    continue;
                }
            }
            out.push(cand);
        }
    }
}

/// Builds continuously chained track & road corridors matching TS `groupConnectedPieces`
/// so multi-segment tracks passing through mountains produce seamless tunnels without fake interior portals.
pub fn build_connected_corridor_chains(
    layout: &Layout,
    catalogs: &[Catalog],
    layer_info: &HashMap<&str, (f64, bool)>,
) -> Vec<CorridorChain> {
    let mut visible: Vec<&TrackPiece> = Vec::new();
    let mut id_to_vidx: HashMap<&str, usize> = HashMap::new();

    for p in &layout.pieces {
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let vis = layer_info.get(lid).map(|x| x.1).unwrap_or(true);
        if vis {
            let vi = visible.len();
            id_to_vidx.insert(p.id.as_str(), vi);
            visible.push(p);
        }
    }

    let is_two_conn_chainable = |p: &TrackPiece| p.connectors.len() == 2;

    let mut out = Vec::new();
    let mut visited_two = vec![false; visible.len()];

    // Pass 1: Chain all 2-connector pieces separately by road vs rail
    for target_road in [false, true] {
        // First start from chain endpoints (connectors NOT leading to another 2-conn piece of same type)
        for pass_loops in [false, true] {
            for start_i in 0..visible.len() {
                if visited_two[start_i] {
                    continue;
                }
                let p_start = visible[start_i];
                if !is_two_conn_chainable(p_start)
                    || is_road_id(catalogs, &p_start.catalog_id) != target_road
                {
                    continue;
                }

                let neighbor_is_chainable = |p: &TrackPiece, c_idx: usize| -> bool {
                    let Some(ref link) = p.connectors[c_idx].connected_to else {
                        return false;
                    };
                    let Some(&ni) = id_to_vidx.get(link.piece_id.as_str()) else {
                        return false;
                    };
                    let np = visible[ni];
                    is_two_conn_chainable(np)
                        && is_road_id(catalogs, &np.catalog_id) == target_road
                        && link.connector_index < 2
                };

                let entry_c = if !pass_loops {
                    if !neighbor_is_chainable(p_start, 0) {
                        0usize
                    } else if !neighbor_is_chainable(p_start, 1) {
                        1usize
                    } else {
                        continue;
                    }
                } else {
                    0usize
                };

                let mut pts_xze = Vec::new();
                let mut cur_idx = start_i;
                let mut cur_entry = entry_c;
                let mut has_catenary = false;
                let mut gauge = 16.5_f64;
                let mut half_w_max = 20.0_f32;
                let start_connected = p_start.connectors[entry_c].connected_to.is_some();
                let mut end_connected = false;

                loop {
                    if visited_two[cur_idx] {
                        break;
                    }
                    visited_two[cur_idx] = true;
                    let cp = visible[cur_idx];
                    let cur_exit = 1 - cur_entry;
                    if cp.has_catenary == Some(true) {
                        has_catenary = true;
                    }
                    let entry_meta = find_entry(catalogs, &cp.catalog_id);
                    if let Some(g) = entry_meta.and_then(|e| e.gauge) {
                        gauge = g;
                    }
                    let hw = if target_road {
                        entry_meta.and_then(|e| e.road_width).unwrap_or(60.0) as f32 * 0.5
                    } else {
                        ((gauge / 16.5) * 20.0) as f32
                    };
                    half_w_max = half_w_max.max(hw);

                    let lid = cp.layer_id.as_deref().unwrap_or("layer-1");
                    let lh = layer_info.get(lid).map(|x| x.0).unwrap_or(0.0);
                    let seg = sample_two_conn_segment_xze(cp, cur_entry, cur_exit, lh);
                    push_dense_subdivided(&mut pts_xze, &seg);

                    end_connected = cp.connectors[cur_exit].connected_to.is_some();
                    let Some(ref link) = cp.connectors[cur_exit].connected_to else {
                        break;
                    };
                    let Some(&next_idx) = id_to_vidx.get(link.piece_id.as_str()) else {
                        break;
                    };
                    let np = visible[next_idx];
                    if visited_two[next_idx]
                        || !is_two_conn_chainable(np)
                        || is_road_id(catalogs, &np.catalog_id) != target_road
                        || link.connector_index >= 2
                    {
                        break;
                    }
                    cur_idx = next_idx;
                    cur_entry = link.connector_index;
                }

                if pts_xze.len() >= 2 {
                    out.push(CorridorChain {
                        pts_xze,
                        half_w: half_w_max,
                        is_road: target_road,
                        has_catenary,
                        gauge,
                        is_turntable: false,
                        start_connected,
                        end_connected,
                    });
                }
            }
        }
    }

    // Pass 2: Standalone, turnouts, crossings & turntables
    for (vi, p) in visible.iter().enumerate() {
        if visited_two[vi] {
            continue;
        }
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let lh = layer_info.get(lid).map(|x| x.0).unwrap_or(0.0);
        let e0 = piece_world_elev(p, lh, 0);
        let e1 = piece_world_elev(p, lh, 1);
        let entry_meta = find_entry(catalogs, &p.catalog_id);
        let is_road = is_road_id(catalogs, &p.catalog_id);
        let gauge = entry_meta.and_then(|e| e.gauge).unwrap_or(16.5);
        let half_w = if is_road {
            entry_meta.and_then(|e| e.road_width).unwrap_or(60.0) as f32 * 0.5
        } else {
            ((gauge / 16.5) * 20.0) as f32
        };
        let has_catenary = p.has_catenary == Some(true);
        let is_turntable = p.connectors.len() >= 6;
        let c = &p.connectors;

        let push_branch = |out: &mut Vec<CorridorChain>, a_idx: usize, b_idx: usize| {
            let ea = piece_world_elev(p, lh, a_idx);
            let eb = piece_world_elev(p, lh, b_idx);
            let pts2d = sample_arc_between_connectors(
                c[a_idx].x,
                c[a_idx].y,
                c[a_idx].angle,
                c[b_idx].x,
                c[b_idx].y,
                c[b_idx].angle,
                16,
            );
            let n = pts2d.len().saturating_sub(1).max(1);
            let raw: Vec<[f32; 3]> = pts2d
                .into_iter()
                .enumerate()
                .map(|(k, pt)| {
                    let u = k as f64 / n as f64;
                    [pt.x as f32, pt.y as f32, (ea + (eb - ea) * u) as f32]
                })
                .collect();
            let mut dense = Vec::new();
            push_dense_subdivided(&mut dense, &raw);
            if dense.len() >= 2 {
                out.push(CorridorChain {
                    pts_xze: dense,
                    half_w,
                    is_road,
                    has_catenary,
                    gauge,
                    is_turntable,
                    start_connected: c[a_idx].connected_to.is_some(),
                    end_connected: c[b_idx].connected_to.is_some(),
                });
            }
        };

        match c.len() {
            0 => {}
            1 => {
                let rad = deg2rad(c[0].angle + 180.0);
                let raw = vec![
                    [c[0].x as f32, c[0].y as f32, e0 as f32],
                    [
                        (c[0].x + rad.cos() * 70.0) as f32,
                        (c[0].y + rad.sin() * 70.0) as f32,
                        e1 as f32,
                    ],
                ];
                let mut dense = Vec::new();
                push_dense_subdivided(&mut dense, &raw);
                out.push(CorridorChain {
                    pts_xze: dense,
                    half_w,
                    is_road,
                    has_catenary,
                    gauge,
                    is_turntable: false,
                    start_connected: c[0].connected_to.is_some(),
                    end_connected: false,
                });
            }
            2 => push_branch(&mut out, 0, 1),
            3 => {
                push_branch(&mut out, 0, 1);
                push_branch(&mut out, 0, 2);
            }
            4 => {
                let a0 = c[0].angle.to_radians();
                let v0x = a0.cos();
                let v0y = a0.sin();
                let facing_opposite = c[1..4]
                    .iter()
                    .filter(|ci| {
                        let ai = ci.angle.to_radians();
                        v0x * ai.cos() + v0y * ai.sin() < -0.15
                    })
                    .count();
                if facing_opposite == 3 {
                    push_branch(&mut out, 0, 1);
                    push_branch(&mut out, 0, 2);
                    push_branch(&mut out, 0, 3);
                } else {
                    push_branch(&mut out, 0, 1);
                    push_branch(&mut out, 2, 3);
                }
            }
            _ => {
                let half = c.len() / 2;
                push_branch(&mut out, 0, half);
            }
        }
    }

    out
}

#[derive(Debug, Clone, Copy)]
pub struct LocalTunnelPt {
    pub lx: f64,
    pub lz: f64,
    pub rel_elev: f64,
    pub norm_r: f64,
}

#[derive(Debug, Clone)]
pub struct TunnelPortalInfo {
    pub at: LocalTunnelPt,
    pub dir_xz: [f64; 2], // unit vector pointing INTO the mountain
    pub prof: TunnelProfile,
    pub suppressed_by_overlap: bool,
}

#[derive(Debug, Clone)]
pub struct TunnelRun {
    pub prof: TunnelProfile,
    pub pts: Vec<LocalTunnelPt>,
    pub start_portal: Option<TunnelPortalInfo>,
    pub end_portal: Option<TunnelPortalInfo>,
}

fn bisect_boundary_crossing(
    a: LocalTunnelPt,
    b: LocalTunnelPt,
    r_portal: f64,
    mountain: &Model3D,
) -> LocalTunnelPt {
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    let a_inside = a.norm_r <= r_portal;
    for _ in 0..10 {
        let t = (lo + hi) * 0.5;
        let lx = a.lx + (b.lx - a.lx) * t;
        let lz = a.lz + (b.lz - a.lz) * t;
        let (nr, _) = evaluate_mountain_local(lx, lz, mountain);
        if (nr <= r_portal) == a_inside {
            lo = t;
        } else {
            hi = t;
        }
    }
    let t = (lo + hi) * 0.5;
    let lx = a.lx + (b.lx - a.lx) * t;
    let lz = a.lz + (b.lz - a.lz) * t;
    let (norm_r, _) = evaluate_mountain_local(lx, lz, mountain);
    LocalTunnelPt {
        lx,
        lz,
        rel_elev: a.rel_elev + (b.rel_elev - a.rel_elev) * t,
        norm_r,
    }
}

fn inward_dir_local(pts: &[LocalTunnelPt], at_start: bool) -> [f64; 2] {
    let base = if at_start { pts[0] } else { *pts.last().unwrap() };
    let iter_pts: Box<dyn Iterator<Item = &LocalTunnelPt>> = if at_start {
        Box::new(pts.iter().skip(1))
    } else {
        Box::new(pts.iter().rev().skip(1))
    };
    for p in iter_pts {
        let dx = p.lx - base.lx;
        let dz = p.lz - base.lz;
        let len = dx.hypot(dz);
        if len >= 4.0 {
            return [dx / len, dz / len];
        }
    }
    let other = if at_start { *pts.last().unwrap() } else { pts[0] };
    let dx = other.lx - base.lx;
    let dz = other.lz - base.lz;
    let len = dx.hypot(dz).max(1e-6);
    [dx / len, dz / len]
}

struct PortalCandidate {
    run_idx: usize,
    is_start: bool,
    at: LocalTunnelPt,
    dir_xz: [f64; 2],
    is_road: bool,
    has_catenary: bool,
    gauge: f64,
    prof: TunnelProfile,
    suppressed_by_overlap: bool,
}

/// Extracts contiguous tunnel spans inside `mountain`, handles boundary crossings,
/// suppresses overlap portals inside neighboring mountains, and clusters parallel multi-track portals.
pub fn compute_tunnel_runs_for_mountain(
    mountain: &Model3D,
    m_base_elev: f64,
    chains: &[CorridorChain],
    all_mountains: &[&Model3D],
    layer_info: &HashMap<&str, (f64, bool)>,
) -> Vec<TunnelRun> {
    let sc = mountain.scale.unwrap_or(1.0).max(0.05);
    let mh = mountain.height.max(20.0) * sc;
    let bound_r = (mountain.width.max(mountain.depth) * sc) * 0.65;
    let cx = mountain.transform.x;
    let cz = mountain.transform.y;

    let mut raw_runs: Vec<(TunnelRun, &CorridorChain)> = Vec::new();

    for ch in chains {
        let near = ch
            .pts_xze
            .iter()
            .any(|p| (p[0] as f64 - cx).hypot(p[1] as f64 - cz) <= bound_r + 120.0);
        if !near || ch.pts_xze.len() < 2 {
            continue;
        }

        let prof = create_tunnel_profile(ch.is_road, ch.has_catenary, ch.gauge, 1, 0.0);
        let mut local_pts: Vec<LocalTunnelPt> = Vec::new();
        for p in &ch.pts_xze {
            let rel_elev = p[2] as f64 - m_base_elev;
            if rel_elev < -35.0 || rel_elev > mh - 10.0 {
                continue;
            }
            let (lx, lz) = world_to_mountain_local(p[0] as f64, p[1] as f64, mountain);
            let (norm_r, _) = evaluate_mountain_local(lx, lz, mountain);
            local_pts.push(LocalTunnelPt {
                lx,
                lz,
                rel_elev,
                norm_r,
            });
        }
        if local_pts.len() < 2 || !local_pts.iter().any(|p| p.norm_r <= ELLIPSE_TOLERANCE) {
            continue;
        }

        struct Span {
            pts: Vec<LocalTunnelPt>,
            has_start_portal: bool,
            has_end_portal: bool,
        }
        let mut spans: Vec<Span> = Vec::new();
        let mut cur: Option<Span> = None;

        for (i, &p) in local_pts.iter().enumerate() {
            if p.norm_r <= R_PORTAL {
                if cur.is_none() {
                    let mut pts = Vec::new();
                    let has_start = if i > 0 {
                        pts.push(bisect_boundary_crossing(
                            local_pts[i - 1],
                            p,
                            R_PORTAL,
                            mountain,
                        ));
                        true
                    } else {
                        !ch.start_connected && p.norm_r >= R_PORTAL - 0.25
                    };
                    cur = Some(Span {
                        pts,
                        has_start_portal: has_start,
                        has_end_portal: false,
                    });
                }
                cur.as_mut().unwrap().pts.push(p);
            } else if let Some(mut s) = cur.take() {
                s.pts.push(bisect_boundary_crossing(
                    local_pts[i - 1],
                    p,
                    R_PORTAL,
                    mountain,
                ));
                s.has_end_portal = true;
                if s.pts.len() >= 2 {
                    spans.push(s);
                }
            }
        }
        if let Some(mut s) = cur.take() {
            if let Some(last) = s.pts.last() {
                if !ch.end_connected && last.norm_r >= R_PORTAL - 0.25 {
                    s.has_end_portal = true;
                }
            }
            if s.pts.len() >= 2 {
                spans.push(s);
            }
        }

        for sp in spans {
            raw_runs.push((
                TunnelRun {
                    prof,
                    pts: sp.pts,
                    start_portal: if sp.has_start_portal {
                        Some(TunnelPortalInfo {
                            at: LocalTunnelPt { lx: 0.0, lz: 0.0, rel_elev: 0.0, norm_r: 0.0 },
                            dir_xz: [0.0, 1.0],
                            prof,
                            suppressed_by_overlap: false,
                        })
                    } else {
                        None
                    },
                    end_portal: if sp.has_end_portal {
                        Some(TunnelPortalInfo {
                            at: LocalTunnelPt { lx: 0.0, lz: 0.0, rel_elev: 0.0, norm_r: 0.0 },
                            dir_xz: [0.0, 1.0],
                            prof,
                            suppressed_by_overlap: false,
                        })
                    } else {
                        None
                    },
                },
                ch,
            ));
        }
    }

    if raw_runs.is_empty() {
        return Vec::new();
    }

    let mut runs: Vec<TunnelRun> = raw_runs.iter().map(|(r, _)| r.clone()).collect();
    let mut candidates: Vec<PortalCandidate> = Vec::new();

    for (run_idx, (run, ch)) in raw_runs.iter().enumerate() {
        if run.start_portal.is_some() {
            let at = run.pts[0];
            if at.rel_elev >= -35.0 && at.rel_elev <= mh - 10.0 {
                let (wx, wy) = mountain_local_to_world(at.lx, at.lz, Some(mountain));
                let suppressed = is_point_inside_other_mountain(
                    wx,
                    wy,
                    m_base_elev + at.rel_elev,
                    &mountain.id,
                    all_mountains,
                    layer_info,
                );
                candidates.push(PortalCandidate {
                    run_idx,
                    is_start: true,
                    at,
                    dir_xz: inward_dir_local(&run.pts, true),
                    is_road: ch.is_road,
                    has_catenary: ch.has_catenary,
                    gauge: ch.gauge,
                    prof: run.prof,
                    suppressed_by_overlap: suppressed,
                });
            } else {
                runs[run_idx].start_portal = None;
            }
        }
        if run.end_portal.is_some() {
            let at = *run.pts.last().unwrap();
            if at.rel_elev >= -35.0 && at.rel_elev <= mh - 10.0 {
                let (wx, wy) = mountain_local_to_world(at.lx, at.lz, Some(mountain));
                let suppressed = is_point_inside_other_mountain(
                    wx,
                    wy,
                    m_base_elev + at.rel_elev,
                    &mountain.id,
                    all_mountains,
                    layer_info,
                );
                candidates.push(PortalCandidate {
                    run_idx,
                    is_start: false,
                    at,
                    dir_xz: inward_dir_local(&run.pts, false),
                    is_road: ch.is_road,
                    has_catenary: ch.has_catenary,
                    gauge: ch.gauge,
                    prof: run.prof,
                    suppressed_by_overlap: suppressed,
                });
            } else {
                runs[run_idx].end_portal = None;
            }
        }
    }

    // Cluster nearby parallel rail portals into single wide multi-track portals
    let mut grouped = vec![false; candidates.len()];
    for i in 0..candidates.len() {
        if grouped[i] {
            continue;
        }
        grouped[i] = true;
        let mut cluster = vec![i];

        if !candidates[i].is_road {
            let g_scale = (candidates[i].gauge / 16.5).clamp(0.4, 3.0);
            let max_group_dist = 115.0 * g_scale;
            for j in (i + 1)..candidates.len() {
                if grouped[j]
                    || candidates[j].is_road
                    || candidates[j].run_idx == candidates[i].run_idx
                {
                    continue;
                }
                let ci = &candidates[i];
                let cj = &candidates[j];
                let dist = (ci.at.lx - cj.at.lx).hypot(ci.at.lz - cj.at.lz);
                let dot = ci.dir_xz[0] * cj.dir_xz[0] + ci.dir_xz[1] * cj.dir_xz[1];
                let elev_diff = (ci.at.rel_elev - cj.at.rel_elev).abs();
                if dist <= max_group_dist && dot > 0.5 && elev_diff < 25.0 {
                    grouped[j] = true;
                    cluster.push(j);
                }
            }
        }

        if cluster.len() == 1 {
            let cand = &candidates[cluster[0]];
            let pinfo = TunnelPortalInfo {
                at: cand.at,
                dir_xz: cand.dir_xz,
                prof: cand.prof,
                suppressed_by_overlap: cand.suppressed_by_overlap,
            };
            if cand.is_start {
                runs[cand.run_idx].start_portal = Some(pinfo);
            } else {
                runs[cand.run_idx].end_portal = Some(pinfo);
            }
        } else {
            let k = cluster.len();
            let kf = k as f64;
            let has_cat = cluster
                .iter()
                .any(|&idx| candidates[idx].has_catenary || candidates[idx].prof.has_catenary);
            let gauge = candidates[cluster[0]].gauge;
            let suppressed = cluster.iter().all(|&idx| candidates[idx].suppressed_by_overlap);

            let avg_lx = cluster.iter().map(|&idx| candidates[idx].at.lx).sum::<f64>() / kf;
            let avg_lz = cluster.iter().map(|&idx| candidates[idx].at.lz).sum::<f64>() / kf;
            let avg_elev = cluster
                .iter()
                .map(|&idx| candidates[idx].at.rel_elev)
                .sum::<f64>()
                / kf;
            let sum_dx = cluster.iter().map(|&idx| candidates[idx].dir_xz[0]).sum::<f64>();
            let sum_dz = cluster.iter().map(|&idx| candidates[idx].dir_xz[1]).sum::<f64>();
            let dlen = sum_dx.hypot(sum_dz).max(1e-6);
            let comb_dir = [sum_dx / dlen, sum_dz / dlen];

            let lat_x = -comb_dir[1];
            let lat_z = comb_dir[0];
            let (mut min_lat, mut max_lat) = (f64::INFINITY, f64::NEG_INFINITY);
            for &idx in &cluster {
                let c_pt = candidates[idx].at;
                let lat = (c_pt.lx - avg_lx) * lat_x + (c_pt.lz - avg_lz) * lat_z;
                min_lat = min_lat.min(lat);
                max_lat = max_lat.max(lat);
            }
            let spread = (max_lat - min_lat).max(0.0);
            let multi_prof = create_tunnel_profile(false, has_cat, gauge, k, spread);
            let (nr, _) = evaluate_mountain_local(avg_lx, avg_lz, mountain);
            let combined_portal = TunnelPortalInfo {
                at: LocalTunnelPt {
                    lx: avg_lx,
                    lz: avg_lz,
                    rel_elev: avg_elev,
                    norm_r: nr,
                },
                dir_xz: comb_dir,
                prof: multi_prof,
                suppressed_by_overlap: suppressed,
            };

            for &idx in &cluster {
                let cand = &candidates[idx];
                if cand.is_start {
                    runs[cand.run_idx].start_portal = Some(combined_portal.clone());
                } else {
                    runs[cand.run_idx].end_portal = Some(combined_portal.clone());
                }
            }
        }
    }

    runs
}

/// Counts intersecting tunnel spans for UI Inspector status display.
pub fn count_mountain_tunnel_collisions(
    mountain: &Model3D,
    layout: &Layout,
    catalogs: &[Catalog],
) -> usize {
    let layer_info = layer_height_visible_map(layout);
    let chains = build_connected_corridor_chains(layout, catalogs, &layer_info);
    let m_lid = mountain.layer_id.as_deref().unwrap_or("layer-1");
    let m_lh = layer_info.get(m_lid).map(|x| x.0).unwrap_or(0.0);
    let m_base = m_lh + mountain.elevation.unwrap_or(0.0);
    let all_mountains: Vec<&Model3D> = layout
        .models
        .as_ref()
        .map(|ms| {
            ms.iter()
                .filter(|m| m.builtin_id.as_deref() == Some("mountain"))
                .collect()
        })
        .unwrap_or_default();
    compute_tunnel_runs_for_mountain(mountain, m_base, &chains, &all_mountains, &layer_info).len()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, create_piece};
    use railbench_core::snapping::update_layout_connections;

    fn empty_test_layout() -> Layout {
        Layout {
            name: "TerrainTest".into(),
            scale: "H0".into(),
            pieces: Vec::new(),
            models: Some(Vec::new()),
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
            baseboard: None,
            baseboards: None,
        }
    }

    #[test]
    fn brush_paint_extend_smooth_and_erase_nodes() {
        let mut layout = empty_test_layout();
        let mut seq = 1u64;
        let mut brush = TerrainBrushState::default();
        brush.set_active(true, Some(TerrainBrushMode::Paint), None);
        brush.radius = 150.0;
        brush.height = 200.0;
        brush.ridge_profile = TerrainRidgeProfile::PeakCenter;

        brush.start_stroke(Vec2 { x: -200.0, y: 0.0 }, &mut layout);
        brush.move_stroke(Vec2 { x: 0.0, y: 80.0 }, &mut layout);
        brush.move_stroke(Vec2 { x: 200.0, y: 0.0 }, &mut layout);
        let id = brush.finish_stroke(&mut layout, &mut seq, None, false).unwrap();

        let m = layout.models.as_ref().unwrap().iter().find(|m| m.id == id).unwrap();
        assert_eq!(m.mountain_path.as_ref().unwrap().len(), 3);
        let mut smoothed = m.clone();
        assert!(smooth_mountain_path(&mut smoothed));

        // Extend existing mountain
        brush.set_active(true, Some(TerrainBrushMode::Extend), Some(id.clone()));
        brush.start_stroke(Vec2 { x: 350.0, y: 0.0 }, &mut layout);
        brush.move_stroke(Vec2 { x: 480.0, y: 0.0 }, &mut layout);
        let ext_id = brush.finish_stroke(&mut layout, &mut seq, None, false).unwrap();
        assert_eq!(ext_id, id);
        assert_eq!(
            layout.models.as_ref().unwrap()[0]
                .mountain_path
                .as_ref()
                .unwrap()
                .len(),
            5
        );

        // Node-accurate erase at x=480 reduces path to 4 nodes
        assert!(erase_terrain_at(Vec2 { x: 480.0, y: 0.0 }, 120.0, &mut layout));
        assert_eq!(
            layout.models.as_ref().unwrap()[0]
                .mountain_path
                .as_ref()
                .unwrap()
                .len(),
            4
        );
    }

    #[test]
    fn brush_ground_mode_creates_brushed_ground_terrain() {
        let mut layout = empty_test_layout();
        let mut seq = 1u64;
        let mut brush = TerrainBrushState::default();
        brush.set_active(true, Some(TerrainBrushMode::Ground), None);
        brush.set_ground_type("water");
        brush.radius = 120.0;

        brush.start_stroke(Vec2 { x: -120.0, y: 0.0 }, &mut layout);
        brush.move_stroke(Vec2 { x: 120.0, y: 40.0 }, &mut layout);
        let id = brush.finish_stroke(&mut layout, &mut seq, None, false).unwrap();

        let m = layout.models.as_ref().unwrap().iter().find(|m| m.id == id).unwrap();
        assert_eq!(m.builtin_id.as_deref(), Some("ground-terrain"));
        assert_eq!(m.ground_terrain_type.as_deref(), Some("water"));
        assert_eq!(m.mountain_path.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn auto_tunnel_marks_intersecting_tracks_and_clears_bridge() {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "24360").unwrap();
        let mut p = create_piece("t1".into(), entry, Transform { x: -180.0, y: 0.0, rotation: 0.0 }, None);
        p.has_bridge = Some(true);

        let mtn = Model3D {
            id: "m1".into(),
            name: "M".into(),
            category: "terrain".into(),
            transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: 600.0,
            depth: 450.0,
            height: 220.0,
            model_type: "builtin".into(),
            builtin_id: Some("mountain".into()),
            mountain_style: Some("alpine".into()),
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };
        let mut layout = empty_test_layout();
        layout.pieces = vec![p];
        layout.models = Some(vec![mtn]);

        let marked = apply_auto_tunnel_all_mountains(&mut layout);
        assert_eq!(marked, 1);
        assert_eq!(layout.pieces[0].has_tunnel, Some(true));
        assert_eq!(layout.pieces[0].has_bridge, Some(false));
    }

    #[test]
    fn multi_track_parallel_tunnels_merge_into_wide_portal() {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "24360").unwrap();
        let a0 = create_piece("a0".into(), entry, Transform { x: -360.0, y: -32.0, rotation: 0.0 }, None);
        let a1 = create_piece("a1".into(), entry, Transform { x: 0.0, y: -32.0, rotation: 0.0 }, None);
        let b0 = create_piece("b0".into(), entry, Transform { x: -360.0, y: 32.0, rotation: 0.0 }, None);
        let b1 = create_piece("b1".into(), entry, Transform { x: 0.0, y: 32.0, rotation: 0.0 }, None);
        let mtn = Model3D {
            id: "m1".into(),
            name: "Twin".into(),
            category: "terrain".into(),
            transform: Transform { x: 0.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: 500.0,
            depth: 500.0,
            height: 240.0,
            model_type: "builtin".into(),
            builtin_id: Some("mountain".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };
        let mut layout = empty_test_layout();
        layout.pieces = vec![a0, a1, b0, b1];
        update_layout_connections(&mut layout, 4.0, 35.0, 1.0);
        layout.models = Some(vec![mtn.clone()]);

        let layer_info = HashMap::new();
        let chains = build_connected_corridor_chains(&layout, &catalogs, &layer_info);
        assert_eq!(chains.len(), 2);
        let runs = compute_tunnel_runs_for_mountain(&mtn, 0.0, &chains, &[&mtn], &layer_info);
        assert_eq!(runs.len(), 2);
        let sp0 = runs[0].start_portal.as_ref().unwrap();
        let sp1 = runs[1].start_portal.as_ref().unwrap();
        assert_eq!(sp0.prof.num_tracks, 2);
        assert_eq!(sp1.prof.num_tracks, 2);
        assert!(sp0.prof.inner_width > 110.0);
    }

    #[test]
    fn overlapping_mountains_suppress_interior_portal() {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "24360").unwrap();
        let p0 = create_piece("p0".into(), entry, Transform { x: -420.0, y: 0.0, rotation: 0.0 }, None);
        let p1 = create_piece("p1".into(), entry, Transform { x: -60.0, y: 0.0, rotation: 0.0 }, None);
        let p2 = create_piece("p2".into(), entry, Transform { x: 300.0, y: 0.0, rotation: 0.0 }, None);

        let m1 = Model3D {
            id: "m1".into(),
            name: "Left".into(),
            category: "terrain".into(),
            transform: Transform { x: -120.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: 440.0,
            depth: 400.0,
            height: 220.0,
            model_type: "builtin".into(),
            builtin_id: Some("mountain".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };
        let m2 = Model3D {
            id: "m2".into(),
            name: "Right".into(),
            category: "terrain".into(),
            transform: Transform { x: 140.0, y: 0.0, rotation: 0.0 },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: None,
            width: 440.0,
            depth: 400.0,
            height: 220.0,
            model_type: "builtin".into(),
            builtin_id: Some("mountain".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        };
        let mut layout = empty_test_layout();
        layout.pieces = vec![p0, p1, p2];
        update_layout_connections(&mut layout, 4.0, 35.0, 1.0);
        layout.models = Some(vec![m1.clone(), m2.clone()]);

        let layer_info = HashMap::new();
        let chains = build_connected_corridor_chains(&layout, &catalogs, &layer_info);
        let runs_m1 = compute_tunnel_runs_for_mountain(&m1, 0.0, &chains, &[&m1, &m2], &layer_info);
        assert_eq!(runs_m1.len(), 1);
        assert!(!runs_m1[0].start_portal.as_ref().unwrap().suppressed_by_overlap);
        assert!(runs_m1[0].end_portal.as_ref().unwrap().suppressed_by_overlap);
    }
}
