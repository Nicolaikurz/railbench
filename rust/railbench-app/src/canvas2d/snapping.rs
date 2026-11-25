use super::*;

pub(super) fn find_building_at(layout: &Layout, wp: Vec2, layer_protection: bool) -> Option<String> {
    let models = layout.models.as_ref()?;
    for m in models.iter().rev() {
        if !is_layer_editable_with_protection(layout, m.layer_id.as_deref(), layer_protection) {
            continue;
        }
        if matches!(m.builtin_id.as_deref(), Some("ground-terrain") | Some("mountain")) {
            continue;
        }
        if model_contains_point(m, wp) {
            return Some(m.id.clone());
        }
    }
    None
}

pub(super) fn find_terrain_at(layout: &Layout, wp: Vec2, layer_protection: bool) -> Option<String> {
    let models = layout.models.as_ref()?;
    for m in models.iter().rev() {
        if !is_layer_editable_with_protection(layout, m.layer_id.as_deref(), layer_protection) {
            continue;
        }
        if !matches!(m.builtin_id.as_deref(), Some("ground-terrain") | Some("mountain")) {
            continue;
        }
        if model_contains_point(m, wp) {
            return Some(m.id.clone());
        }
    }
    None
}

pub(super) fn find_piece_at(
    layout: &Layout,
    cache: &Canvas2DCache,
    wp: Vec2,
    tolerance: f64,
    layer_protection: bool,
) -> Option<String> {
    let tol_sq = tolerance * tolerance;
    let mut best_idx: Option<usize> = None;
    let mut best_d_sq = f64::INFINITY;
    let active_lid = layout.active_layer_id.as_deref().unwrap_or("layer-1");
    for (idx, p) in layout.pieces.iter().enumerate() {
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        if layer_protection && lid != active_lid {
            continue;
        }
        let editable = cache
            .layer_flags
            .get(lid)
            .map_or_else(|| is_layer_editable(layout, Some(lid)), |&(_, e)| e);
        if !editable {
            continue;
        }
        let g = &cache.pieces[idx];
        if !point_in_padded_aabb((g.min_x, g.min_y, g.max_x, g.max_y), wp.x, wp.y, tolerance) {
            continue;
        }
        for branch in &g.branches {
            for w in branch.windows(2) {
                let d_sq = point_to_segment_dist_sq(wp, w[0], w[1]);
                if d_sq <= tol_sq && d_sq < best_d_sq {
                    best_d_sq = d_sq;
                    best_idx = Some(idx);
                }
            }
        }
        let area_d_sq = (tolerance * 0.65).powi(2);
        let inside_shape = match g.hit_shape {
            PieceHitShape::BranchesOnly => false,
            PieceHitShape::Circle { cx, cy, radius } => {
                (wp.x - cx).hypot(wp.y - cy) <= radius + tolerance * 0.35
            }
            PieceHitShape::OrientedRect {
                origin,
                rot_deg,
                min_x,
                max_x,
                min_y,
                max_y,
            } => {
                let loc = rotate(
                    Vec2 {
                        x: wp.x - origin.x,
                        y: wp.y - origin.y,
                    },
                    -rot_deg,
                );
                let pad = tolerance * 0.35;
                loc.x >= min_x - pad
                    && loc.x <= max_x + pad
                    && loc.y >= min_y - pad
                    && loc.y <= max_y + pad
            }
        };
        if inside_shape && area_d_sq < best_d_sq {
            best_d_sq = area_d_sq;
            best_idx = Some(idx);
        }
    }
    best_idx.map(|i| layout.pieces[i].id.clone())
}

#[inline]
pub(super) fn point_to_segment_dist_sq(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    let len_sq = vx * vx + vy * vy;
    if len_sq < 1e-6 {
        let dx = p.x - a.x;
        let dy = p.y - a.y;
        return dx * dx + dy * dy;
    }
    let t = (((p.x - a.x) * vx + (p.y - a.y) * vy) / len_sq).clamp(0.0, 1.0);
    let px = a.x + t * vx;
    let py = a.y + t * vy;
    let dx = p.x - px;
    let dy = p.y - py;
    dx * dx + dy * dy
}

pub(super) struct SnapCandidate {
    pub(super) src_x: f64,
    pub(super) src_y: f64,
    pub(super) src_angle: f64,
    pub(super) dst_x: f64,
    pub(super) dst_y: f64,
    pub(super) dst_angle: f64,
    pub(super) same_level: bool,
}

pub(super) fn find_best_snap_candidate(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    max_dist: f64,
) -> Option<SnapCandidate> {
    find_best_snap_candidate_internal(layout, catalogs, selected, max_dist, true)
}

pub(super) fn find_best_snap_candidate_any_level(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    max_dist: f64,
) -> Option<SnapCandidate> {
    find_best_snap_candidate_internal(layout, catalogs, selected, max_dist, false)
}

pub(super) fn find_best_snap_candidate_internal(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    max_dist: f64,
    require_same_level: bool,
) -> Option<SnapCandidate> {
    if selected.is_empty() || max_dist <= 0.0 {
        return None;
    }

    let mut layer_vis: HashMap<&str, bool> = HashMap::new();
    let mut layer_heights: HashMap<&str, f64> = HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            layer_vis.insert(l.id.as_str(), l.visible);
            layer_heights.insert(l.id.as_str(), l.height_mm.unwrap_or(0.0));
        }
    }

    let cell_size = (max_dist * 1.05 + 1.0).max(16.0);
    let inv_cell = 1.0 / cell_size;
    let max_dist_sq = max_dist * max_dist;

    let mut open_grid: HashMap<(i32, i32), Vec<(f64, f64, f64, f64, bool)>> =
        HashMap::with_capacity(layout.pieces.len() * 2);

    for other in &layout.pieces {
        if selected.contains(&other.id) {
            continue;
        }
        let lid = other.layer_id.as_deref().unwrap_or("layer-1");
        if !layer_vis.get(lid).copied().unwrap_or(true) {
            continue;
        }
        let other_lh = layer_heights.get(lid).copied().unwrap_or(0.0);
        let other_is_road = crate::catalog::is_road_id(catalogs, &other.catalog_id);
        for (c2_idx, c2) in other.connectors.iter().enumerate() {
            // Treat connectors connected only to a dragged piece as open too
            let is_open = match &c2.connected_to {
                None => true,
                Some(r) => selected.contains(&r.piece_id),
            };
            if !is_open {
                continue;
            }
            let h2 = railbench_core::snapping::piece_connector_height(other, c2_idx, other_lh);
            let gx = (c2.x * inv_cell).floor() as i32;
            let gy = (c2.y * inv_cell).floor() as i32;
            open_grid
                .entry((gx, gy))
                .or_default()
                .push((c2.x, c2.y, c2.angle, h2, other_is_road));
        }
    }

    if open_grid.is_empty() {
        return None;
    }

    let mut best_same: Option<(f64, SnapCandidate)> = None;
    let mut best_diff: Option<(f64, SnapCandidate)> = None;
    for p in &layout.pieces {
        if !selected.contains(&p.id) {
            continue;
        }
        let p_lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let p_lh = layer_heights.get(p_lid).copied().unwrap_or(0.0);
        let p_is_road = crate::catalog::is_road_id(catalogs, &p.catalog_id);
        for (c1_idx, c1) in p.connectors.iter().enumerate() {
            let h1 = railbench_core::snapping::piece_connector_height(p, c1_idx, p_lh);
            let gx = (c1.x * inv_cell).floor() as i32;
            let gy = (c1.y * inv_cell).floor() as i32;
            for ox in -1..=1 {
                for oy in -1..=1 {
                    let Some(bucket) = open_grid.get(&(gx + ox, gy + oy)) else {
                        continue;
                    };
                    for &(dst_x, dst_y, dst_angle, dst_h, other_is_road) in bucket {
                        // Never snap tracks to roads or roads to tracks!
                        if other_is_road != p_is_road {
                            continue;
                        }
                        let same_level = (h1 - dst_h).abs() <= 1.0;
                        if require_same_level && !same_level {
                            continue;
                        }
                        let dx = c1.x - dst_x;
                        let dy = c1.y - dst_y;
                        let d_sq = dx * dx + dy * dy;
                        if d_sq <= max_dist_sq {
                            let cand = SnapCandidate {
                                src_x: c1.x,
                                src_y: c1.y,
                                src_angle: c1.angle,
                                dst_x,
                                dst_y,
                                dst_angle,
                                same_level,
                            };
                            if same_level {
                                if best_same.as_ref().map_or(true, |(bd_sq, _)| d_sq < *bd_sq) {
                                    best_same = Some((d_sq, cand));
                                }
                            } else if best_diff.as_ref().map_or(true, |(bd_sq, _)| d_sq < *bd_sq) {
                                best_diff = Some((d_sq, cand));
                            }
                        }
                    }
                }
            }
        }
    }
    best_same.or(best_diff).map(|(_, cand)| cand)
}

/// Finds the nearest visible, open, same-type, same-level track connector to `cursor_world`
/// for snapping a flex track endpoint (`flex_id`, `moving_conn_idx`).
pub(super) fn find_best_flex_endpoint_snap(
    layout: &Layout,
    catalogs: &[Catalog],
    flex_id: &str,
    moving_conn_idx: usize,
    cursor_world: Vec2,
    max_dist: f64,
) -> Option<(Vec2, f64)> {
    find_best_flex_endpoint_snap_with_height(
        layout,
        catalogs,
        flex_id,
        moving_conn_idx,
        cursor_world,
        max_dist,
    )
    .map(|(p, a, _)| (p, a))
}

pub(super) fn find_best_flex_endpoint_snap_with_height(
    layout: &Layout,
    catalogs: &[Catalog],
    flex_id: &str,
    moving_conn_idx: usize,
    cursor_world: Vec2,
    max_dist: f64,
) -> Option<(Vec2, f64, f64)> {
    if max_dist <= 0.0 {
        return None;
    }
    let flex_piece = layout.pieces.iter().find(|p| p.id == flex_id)?;
    if moving_conn_idx >= flex_piece.connectors.len() {
        return None;
    }

    let mut layer_vis: HashMap<&str, bool> = HashMap::new();
    let mut layer_heights: HashMap<&str, f64> = HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            layer_vis.insert(l.id.as_str(), l.visible);
            layer_heights.insert(l.id.as_str(), l.height_mm.unwrap_or(0.0));
        }
    }

    let flex_lid = flex_piece.layer_id.as_deref().unwrap_or("layer-1");
    let flex_lh = layer_heights.get(flex_lid).copied().unwrap_or(0.0);
    let flex_h = railbench_core::snapping::piece_connector_height(flex_piece, moving_conn_idx, flex_lh);
    let flex_is_road = crate::catalog::is_road_id(catalogs, &flex_piece.catalog_id);

    let max_dist_sq = max_dist * max_dist;
    let mut best_same: Option<(f64, Vec2, f64, f64)> = None;
    let mut best_ramp: Option<(f64, Vec2, f64, f64)> = None;

    for other in &layout.pieces {
        if other.id == flex_id {
            continue;
        }
        let lid = other.layer_id.as_deref().unwrap_or("layer-1");
        if !layer_vis.get(lid).copied().unwrap_or(true) {
            continue;
        }
        if crate::catalog::is_road_id(catalogs, &other.catalog_id) != flex_is_road {
            continue;
        }
        let other_lh = layer_heights.get(lid).copied().unwrap_or(0.0);
        for (c2_idx, c2) in other.connectors.iter().enumerate() {
            let is_open = match &c2.connected_to {
                None => true,
                Some(r) => r.piece_id == flex_id,
            };
            if !is_open {
                continue;
            }
            let h2 = railbench_core::snapping::piece_connector_height(other, c2_idx, other_lh);
            let ddx = cursor_world.x - c2.x;
            let ddy = cursor_world.y - c2.y;
            let d2 = ddx * ddx + ddy * ddy;
            if d2 <= max_dist_sq {
                if (flex_h - h2).abs() <= 1.5 {
                    if best_same.map_or(true, |(bd2, _, _, _)| d2 < bd2) {
                        best_same = Some((d2, Vec2 { x: c2.x, y: c2.y }, c2.angle, h2));
                    }
                } else if best_ramp.map_or(true, |(bd2, _, _, _)| d2 < bd2) {
                    best_ramp = Some((d2, Vec2 { x: c2.x, y: c2.y }, c2.angle, h2));
                }
            }
        }
    }

    best_same.or(best_ramp).map(|(_, pos, ang, h)| (pos, ang, h))
}

pub fn snap_selected_pieces(
    layout: &mut Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    max_dist: f64,
) -> bool {
    let Some(cand) = find_best_snap_candidate(layout, catalogs, selected, max_dist) else {
        return false;
    };

    // Exact rigid-body rotation around (src_x, src_y) + translation to (dst_x, dst_y).
    // Works for 100% of track types (curves, turnouts, flextracks, multi-selection) without catalog lookup.
    let delta_rot = norm_angle_deg(cand.dst_angle + 180.0 - cand.src_angle);
    let pivot = Vec2 {
        x: cand.src_x,
        y: cand.src_y,
    };
    let shift_x = cand.dst_x - cand.src_x;
    let shift_y = cand.dst_y - cand.src_y;

    for p in &mut layout.pieces {
        if !selected.contains(&p.id) {
            continue;
        }
        let rel_t = rotate(
            Vec2 {
                x: p.transform.x - pivot.x,
                y: p.transform.y - pivot.y,
            },
            delta_rot,
        );
        p.transform.x = pivot.x + rel_t.x + shift_x;
        p.transform.y = pivot.y + rel_t.y + shift_y;
        p.transform.rotation = norm_angle_deg(p.transform.rotation + delta_rot);

        for c in &mut p.connectors {
            let rel_c = rotate(
                Vec2 {
                    x: c.x - pivot.x,
                    y: c.y - pivot.y,
                },
                delta_rot,
            );
            c.x = pivot.x + rel_c.x + shift_x;
            c.y = pivot.y + rel_c.y + shift_y;
            c.angle = norm_angle_deg(c.angle + delta_rot);
            c.connected_to = None;
        }
    }

    // Flex-track dual-end bridging: if a single flex track just snapped one end onto Track A
    // and its other end landed near an open connector on Track B, automatically bend & shorten
    // the flex track to snap cleanly onto Track B as well (0° tangent kink at both ends).
    if selected.len() == 1 {
        if let Some(flex_id) = selected.iter().next() {
            let flex_info = layout.pieces.iter().find(|p| &p.id == flex_id).and_then(|p| {
                if p.connectors.len() == 2 && crate::catalog::is_flex_id(catalogs, &p.catalog_id) {
                    // Determine which connector just snapped to (cand.dst_x, cand.dst_y)
                    let d0 = (p.connectors[0].x - cand.dst_x).hypot(p.connectors[0].y - cand.dst_y);
                    let d1 = (p.connectors[1].x - cand.dst_x).hypot(p.connectors[1].y - cand.dst_y);
                    let other_idx = if d0 <= d1 { 1 } else { 0 };
                    let other_pos = Vec2 {
                        x: p.connectors[other_idx].x,
                        y: p.connectors[other_idx].y,
                    };
                    Some((flex_id.clone(), other_idx, other_pos))
                } else {
                    None
                }
            });
            if let Some((fid, other_idx, other_pos)) = flex_info {
                let bridge_dist = (max_dist * 1.6).max(32.0);
                if let Some((tpos, tang)) =
                    find_best_flex_endpoint_snap(layout, catalogs, &fid, other_idx, other_pos, bridge_dist)
                {
                    if let Some(p) = layout.pieces.iter_mut().find(|pc| pc.id == fid) {
                        let _ = crate::catalog::snap_flex_endpoint(p, catalogs, other_idx, tpos, tang);
                    }
                }
            }
        }
    }
    true
}

