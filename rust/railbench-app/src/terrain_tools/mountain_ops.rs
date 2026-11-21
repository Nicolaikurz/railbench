use super::*;

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

pub(super) fn is_point_inside_terrain_model(wp: Vec2, m: &Model3D) -> bool {
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

pub(super) fn layer_height_visible_map(layout: &Layout) -> HashMap<&str, (f64, bool)> {
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

