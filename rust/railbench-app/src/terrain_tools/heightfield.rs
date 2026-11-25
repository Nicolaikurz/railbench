use std::sync::Arc;

use railbench_core::geometry::{is_point_in_any_polygon, smoothstep};
use railbench_core::terrain::{
    create_heightfield, resample_heightfield, ridged_fbm_noise, sample_heightfield_bilinear,
    BIOME_AUTO, BIOME_GRASS, BIOME_WATER,
};
use railbench_core::types::{BaseboardConfig, Layout, TerrainHeightfield, TrackPiece, Vec2};

use crate::board_tools;
use crate::export::sample_track_branches_world;

use super::{
    collect_terrain_support_polygons, erase_terrain_at_on_layer, pack_support_polygons,
};

pub const DEFAULT_CELL_MM: f64 = 26.0;
pub const MIN_HEIGHT_MM: f32 = -32.0;
pub const MAX_HEIGHT_MM: f32 = 560.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SculptOp {
    Raise,
    Lower,
    Smooth,
    Flatten(f32),
    GradeTrack,
    PaintBiome(u8),
    Reset,
}

/// Returns `(active_layer_id, active_height_mm, active_visible_and_unlocked, active_layer_name)`.
pub fn active_layer_info(layout: &Layout) -> (Option<String>, f64, bool, String) {
    if let Some(layers) = layout.layers.as_ref().filter(|ls| !ls.is_empty()) {
        let lay = layout
            .active_layer_id
            .as_deref()
            .and_then(|aid| layers.iter().find(|l| l.id == aid))
            .unwrap_or(&layers[0]);
        return (
            Some(lay.id.clone()),
            lay.height_mm.unwrap_or(0.0),
            lay.visible && !lay.locked,
            lay.name.clone(),
        );
    }
    (None, 0.0, true, "Base".into())
}

/// Tight world bounding box `(min_x, min_z, max_x, max_z)` across all enabled baseboards.
pub fn board_world_bounds(layout: &Layout) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for b in board_tools::get_baseboards_slice(layout) {
        if !b.enabled {
            continue;
        }
        for pt in board_tools::get_baseboard_polygon(&b) {
            min_x = min_x.min(pt.x);
            min_z = min_z.min(pt.y);
            max_x = max_x.max(pt.x);
            max_z = max_z.max(pt.y);
        }
    }
    if !min_x.is_finite() || (max_x - min_x) < 10.0 || (max_z - min_z) < 10.0 {
        (-1200.0, -600.0, 1200.0, 600.0)
    } else {
        (min_x, min_z, max_x, max_z)
    }
}

/// Tight world bounding box `(min_x, min_z, max_x, max_z)` of the active layer's support plates/boards.
pub fn active_layer_bounds(layout: &Layout) -> (f64, f64, f64, f64) {
    let (active_lid, _, _, _) = active_layer_info(layout);
    let boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
        .into_iter()
        .filter(|b| b.enabled)
        .collect();
    let polys = collect_terrain_support_polygons(layout, active_lid.as_deref(), &boards);
    let mut min_x = f64::INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for poly in &polys {
        for pt in poly {
            min_x = min_x.min(pt.x);
            min_z = min_z.min(pt.y);
            max_x = max_x.max(pt.x);
            max_z = max_z.max(pt.y);
        }
    }
    if !min_x.is_finite() || (max_x - min_x) < 10.0 || (max_z - min_z) < 10.0 {
        board_world_bounds(layout)
    } else {
        (min_x, min_z, max_x, max_z)
    }
}

/// Returns a read-only reference to the active layer's `TerrainHeightfield` (if present).
pub fn active_layer_heightfield(layout: &Layout) -> Option<&TerrainHeightfield> {
    if let Some(layers) = layout.layers.as_ref().filter(|ls| !ls.is_empty()) {
        let (active_lid, _, _, _) = active_layer_info(layout);
        let idx = layers
            .iter()
            .position(|l| Some(l.id.as_str()) == active_lid.as_deref())
            .unwrap_or(0);
        if let Some(ref hf) = layers[idx].terrain_field {
            return Some(hf);
        }
        if idx == 0 {
            return layout.terrain_field.as_ref();
        }
        return None;
    }
    layout.terrain_field.as_ref()
}

fn ensure_hf_slot(
    slot: &mut Option<TerrainHeightfield>,
    min_x: f64,
    min_z: f64,
    max_x: f64,
    max_z: f64,
) -> &mut TerrainHeightfield {
    let needs_resize = slot.as_ref().map_or(true, |hf| {
        (hf.min_x - min_x).abs() > 25.0
            || (hf.min_z - min_z).abs() > 25.0
            || (hf.max_x - max_x).abs() > 25.0
            || (hf.max_z - max_z).abs() > 25.0
            || hf.cols < 2
            || hf.rows < 2
            || hf.heights.len() != hf.cols * hf.rows
            || hf.materials.len() != hf.cols * hf.rows
    });
    if needs_resize {
        let updated = match slot.as_ref() {
            Some(old) if old.cols >= 2 && old.rows >= 2 => {
                resample_heightfield(old, min_x, min_z, max_x, max_z, DEFAULT_CELL_MM)
            }
            _ => create_heightfield(min_x, min_z, max_x, max_z, DEFAULT_CELL_MM),
        };
        *slot = Some(updated);
    }
    slot.as_mut().unwrap()
}

/// Ensures the active layer has an up-to-date `TerrainHeightfield` sized to its support polygons.
/// Returns `None` if the active layer is hidden or locked.
pub fn ensure_active_layer_heightfield(layout: &mut Layout) -> Option<&mut TerrainHeightfield> {
    let (active_lid, _, editable, _) = active_layer_info(layout);
    if !editable {
        return None;
    }
    let (min_x, min_z, max_x, max_z) = active_layer_bounds(layout);
    if let Some(layers) = layout.layers.as_mut().filter(|ls| !ls.is_empty()) {
        let idx = layers
            .iter()
            .position(|l| Some(l.id.as_str()) == active_lid.as_deref())
            .unwrap_or(0);
        if idx == 0 && layers[0].terrain_field.is_none() && layout.terrain_field.is_some() {
            layers[0].terrain_field = layout.terrain_field.take();
        }
        return Some(ensure_hf_slot(
            &mut layers[idx].terrain_field,
            min_x,
            min_z,
            max_x,
            max_z,
        ));
    }
    Some(ensure_hf_slot(
        &mut layout.terrain_field,
        min_x,
        min_z,
        max_x,
        max_z,
    ))
}

fn collect_track_elevation_samples(
    layout: &Layout,
    active_lid: Option<&str>,
    active_layer_y0: f64,
    center: Vec2,
    reach_r: f64,
) -> Vec<(f64, f64, f32)> {
    let reach_pad = reach_r + 160.0;
    let mut samples = Vec::new();
    for piece in &layout.pieces {
        if piece.has_bridge == Some(true) {
            continue;
        }
        let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
        if let Some(target_lid) = active_lid {
            if lid != target_lid {
                continue;
            }
        }
        // Elevations relative to the active layer's wood surface (y = active_layer_y0)
        let e0 = piece.elevation.map(|e| e.start).unwrap_or(0.0);
        let e1 = piece
            .elevation
            .map(|e| e.end)
            .unwrap_or_else(|| piece.elevation.map(|e| e.start).unwrap_or(0.0));
        let _ = active_layer_y0;
        if let Some(c0) = piece.connectors.first() {
            if (c0.x - center.x).hypot(c0.y - center.y) > reach_pad * 3.0 {
                let any_near = piece
                    .connectors
                    .iter()
                    .any(|c| (c.x - center.x).hypot(c.y - center.y) <= reach_pad * 2.5);
                if !any_near {
                    continue;
                }
            }
        }
        for c in &piece.connectors {
            let h_c = if piece.connectors.len() >= 2 {
                let d0 = (c.x - piece.connectors[0].x).hypot(c.y - piece.connectors[0].y);
                let d1 = (c.x - piece.connectors[1].x).hypot(c.y - piece.connectors[1].y);
                if d0 + d1 > 1.0 {
                    e0 + (e1 - e0) * (d0 / (d0 + d1))
                } else {
                    e0
                }
            } else {
                e0
            };
            if (c.x - center.x).hypot(c.y - center.y) <= reach_pad {
                samples.push((c.x, c.y, h_c as f32));
            }
        }
        let b0 = piece
            .connectors
            .first()
            .map(|c| (c.x, c.y))
            .unwrap_or((piece.transform.x, piece.transform.y));
        let b1 = piece.connectors.get(1).map(|c| (c.x, c.y)).unwrap_or(b0);
        let seg_len = (b1.0 - b0.0).hypot(b1.1 - b0.1).max(1.0);
        for k in 1..6 {
            let t = k as f64 / 6.0;
            let sx = b0.0 + (b1.0 - b0.0) * t;
            let sz = b0.1 + (b1.1 - b0.1) * t;
            if (sx - center.x).hypot(sz - center.y) <= reach_pad + seg_len * 0.2 {
                samples.push((sx, sz, (e0 + (e1 - e0) * t) as f32));
            }
        }
    }
    samples
}

/// Applies one brush dab of `op` at `center` onto ONLY the active layer's heightfield,
/// strictly clipped to that layer's support plates/baseboards.
pub fn apply_heightfield_brush(
    layout: &mut Layout,
    op: SculptOp,
    center: Vec2,
    radius_mm: f64,
    strength: f32,
    max_height_mm: f32,
    paint_biome: u8,
) -> bool {
    let (active_lid, active_y0, editable, _) = active_layer_info(layout);
    if !editable {
        return false;
    }

    let radius = radius_mm.clamp(30.0, 700.0);
    let strength = strength.clamp(0.05, 1.0);
    let max_h = max_height_mm.clamp(20.0, MAX_HEIGHT_MM);

    let boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
        .into_iter()
        .filter(|b| b.enabled)
        .collect();
    let support_polys = collect_terrain_support_polygons(layout, active_lid.as_deref(), &boards);
    let clip_support = !support_polys.is_empty();
    let (poly_offsets, poly_coords) = pack_support_polygons(&support_polys);
    let track_samples = if op == SculptOp::GradeTrack {
        collect_track_elevation_samples(layout, active_lid.as_deref(), active_y0, center, radius)
    } else {
        Vec::new()
    };
    if op == SculptOp::GradeTrack && track_samples.is_empty() {
        return false;
    }

    let mut legacy_erased = false;
    if op == SculptOp::Reset {
        legacy_erased = erase_terrain_at_on_layer(center, radius, layout, active_lid.as_deref());
    }

    let Some(hf) = ensure_active_layer_heightfield(layout) else {
        return legacy_erased;
    };
    let cols = hf.cols;
    let rows = hf.rows;
    let span_x = (hf.max_x - hf.min_x).max(1.0);
    let span_z = (hf.max_z - hf.min_z).max(1.0);
    let dx = span_x / (cols - 1) as f64;
    let dz = span_z / (rows - 1) as f64;

    let ix0 = (((center.x - radius - hf.min_x) / dx).floor() as isize).clamp(0, cols as isize - 1) as usize;
    let ix1 = (((center.x + radius - hf.min_x) / dx).ceil() as isize).clamp(0, cols as isize - 1) as usize;
    let iz0 = (((center.y - radius - hf.min_z) / dz).floor() as isize).clamp(0, rows as isize - 1) as usize;
    let iz1 = (((center.y + radius - hf.min_z) / dz).ceil() as isize).clamp(0, rows as isize - 1) as usize;

    let mut smooth_snapshot: Option<Vec<f32>> = None;
    if op == SculptOp::Smooth {
        smooth_snapshot = Some(hf.heights.as_ref().clone());
    }

    let heights = Arc::make_mut(&mut hf.heights);
    let materials = Arc::make_mut(&mut hf.materials);
    let mut changed = legacy_erased;

    for iz in iz0..=iz1 {
        let wz = hf.min_z + iz as f64 * dz;
        for ix in ix0..=ix1 {
            let wx = hf.min_x + ix as f64 * dx;
            let dist = (wx - center.x).hypot(wz - center.y);
            if dist > radius {
                continue;
            }
            if clip_support && !is_point_in_any_polygon(wx, wz, &poly_offsets, &poly_coords) {
                continue;
            }

            let w = (1.0 - smoothstep(0.0, 1.0, dist / radius)) as f32;
            if w <= 1e-4 {
                continue;
            }
            let idx = iz * cols + ix;
            let old_h = heights[idx];
            let old_m = materials[idx];

            match op {
                SculptOp::Raise => {
                    let n = ridged_fbm_noise(wx / 190.0, wz / 190.0) as f32;
                    let delta = strength * w * (18.0 + 10.0 * n);
                    heights[idx] = (old_h + delta).min(max_h);
                    if paint_biome != BIOME_AUTO && w > 0.35 {
                        materials[idx] = paint_biome;
                    }
                }
                SculptOp::Lower => {
                    let delta = strength * w * 18.0;
                    heights[idx] = (old_h - delta).max(MIN_HEIGHT_MM);
                }
                SculptOp::Smooth => {
                    if let Some(ref snap) = smooth_snapshot {
                        let mut sum = 0.0_f32;
                        let mut wsum = 0.0_f32;
                        for oz in -1isize..=1 {
                            let nz = (iz as isize + oz).clamp(0, rows as isize - 1) as usize;
                            for ox in -1isize..=1 {
                                let nx = (ix as isize + ox).clamp(0, cols as isize - 1) as usize;
                                let kw = match (ox == 0, oz == 0) {
                                    (true, true) => 4.0,
                                    (true, false) | (false, true) => 2.0,
                                    _ => 1.0,
                                };
                                sum += snap[nz * cols + nx] * kw;
                                wsum += kw;
                            }
                        }
                        let avg = sum / wsum;
                        let blend = (strength * 0.65 * w).clamp(0.0, 0.85);
                        heights[idx] = old_h + (avg - old_h) * blend;
                    }
                }
                SculptOp::Flatten(target_h) => {
                    let target = target_h.clamp(MIN_HEIGHT_MM, max_h);
                    let blend = (strength * 0.65 * w).clamp(0.0, 0.90);
                    heights[idx] = old_h + (target - old_h) * blend;
                }
                SculptOp::GradeTrack => {
                    let mut best_d = f64::INFINITY;
                    let mut best_e = 0.0_f32;
                    for &(sx, sz, se) in &track_samples {
                        let d = (wx - sx).hypot(wz - sz);
                        if d < best_d {
                            best_d = d;
                            best_e = se;
                        }
                    }
                    if best_d <= radius + 40.0 {
                        let track_bed = (best_e - 1.5).max(0.0);
                        let slope_drop = ((best_d - 28.0).max(0.0) as f32) * 0.72;
                        let target = (track_bed - slope_drop).max(0.0).min(max_h);
                        let blend = (strength * 0.72 * w).clamp(0.0, 0.92);
                        heights[idx] = old_h + (target - old_h) * blend;
                    }
                }
                SculptOp::PaintBiome(b) => {
                    if w > 0.28 {
                        materials[idx] = b;
                    }
                    if b == BIOME_WATER && old_h <= 14.0 {
                        let target = -6.0_f32;
                        heights[idx] =
                            old_h + (target - old_h) * (strength * 0.55 * w).clamp(0.0, 0.85);
                    }
                }
                SculptOp::Reset => {
                    let next_h = old_h * (1.0 - (strength * 0.75 * w).clamp(0.0, 1.0));
                    heights[idx] = if next_h.abs() < 1.5 { 0.0 } else { next_h };
                    if w > 0.25 {
                        materials[idx] = BIOME_AUTO;
                    }
                }
            }

            if (heights[idx] - old_h).abs() > 1e-4 || materials[idx] != old_m {
                changed = true;
            }
        }
    }

    if changed {
        hf.revision = hf.revision.wrapping_add(1);
    }
    changed
}

/// Fills all supported heightfield vertices on the active layer with a uniform biome material.
pub fn fill_heightfield_biome(layout: &mut Layout, biome_id: u8) -> bool {
    let (active_lid, _, editable, _) = active_layer_info(layout);
    if !editable {
        return false;
    }
    let boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
        .into_iter()
        .filter(|b| b.enabled)
        .collect();
    let support_polys = collect_terrain_support_polygons(layout, active_lid.as_deref(), &boards);
    let clip_support = !support_polys.is_empty();
    let (poly_offsets, poly_coords) = pack_support_polygons(&support_polys);

    let Some(hf) = ensure_active_layer_heightfield(layout) else {
        return false;
    };
    let cols = hf.cols;
    let rows = hf.rows;
    let span_x = (hf.max_x - hf.min_x).max(1.0);
    let span_z = (hf.max_z - hf.min_z).max(1.0);
    let mats = Arc::make_mut(&mut hf.materials);
    let mut changed = false;

    for iz in 0..rows {
        let wz = hf.min_z + (iz as f64 / (rows - 1).max(1) as f64) * span_z;
        for ix in 0..cols {
            let wx = hf.min_x + (ix as f64 / (cols - 1).max(1) as f64) * span_x;
            if clip_support && !is_point_in_any_polygon(wx, wz, &poly_offsets, &poly_coords) {
                continue;
            }
            let idx = iz * cols + ix;
            if mats[idx] != biome_id {
                mats[idx] = biome_id;
                changed = true;
            }
        }
    }
    if changed {
        hf.revision = hf.revision.wrapping_add(1);
    }
    changed
}

/// Runs one smoothing pass over the active layer's heightfield within its support polygons.
pub fn smooth_entire_heightfield(layout: &mut Layout) -> bool {
    let (active_lid, _, editable, _) = active_layer_info(layout);
    if !editable {
        return false;
    }
    let (cols, rows, snap) = {
        let Some(hf) = active_layer_heightfield(layout) else {
            return false;
        };
        if hf.cols < 3 || hf.rows < 3 {
            return false;
        }
        (hf.cols, hf.rows, hf.heights.as_ref().clone())
    };
    let boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
        .into_iter()
        .filter(|b| b.enabled)
        .collect();
    let support_polys = collect_terrain_support_polygons(layout, active_lid.as_deref(), &boards);
    let clip_support = !support_polys.is_empty();
    let (poly_offsets, poly_coords) = pack_support_polygons(&support_polys);

    let Some(hf_mut) = ensure_active_layer_heightfield(layout) else {
        return false;
    };
    let span_x = (hf_mut.max_x - hf_mut.min_x).max(1.0);
    let span_z = (hf_mut.max_z - hf_mut.min_z).max(1.0);
    let heights = Arc::make_mut(&mut hf_mut.heights);
    let mut changed = false;

    for iz in 0..rows {
        let wz = hf_mut.min_z + (iz as f64 / (rows - 1) as f64) * span_z;
        for ix in 0..cols {
            let wx = hf_mut.min_x + (ix as f64 / (cols - 1).max(1) as f64) * span_x;
            if clip_support && !is_point_in_any_polygon(wx, wz, &poly_offsets, &poly_coords) {
                continue;
            }
            let mut sum = 0.0_f32;
            let mut wsum = 0.0_f32;
            for oz in -1isize..=1 {
                let nz = (iz as isize + oz).clamp(0, rows as isize - 1) as usize;
                for ox in -1isize..=1 {
                    let nx = (ix as isize + ox).clamp(0, cols as isize - 1) as usize;
                    let kw = match (ox == 0, oz == 0) {
                        (true, true) => 4.0,
                        (true, false) | (false, true) => 2.0,
                        _ => 1.0,
                    };
                    sum += snap[nz * cols + nx] * kw;
                    wsum += kw;
                }
            }
            let next_h = sum / wsum;
            let idx = iz * cols + ix;
            if (next_h - heights[idx]).abs() > 1e-4 {
                heights[idx] = next_h;
                changed = true;
            }
        }
    }
    if changed {
        hf_mut.revision = hf_mut.revision.wrapping_add(1);
    }
    changed
}

/// Clears ONLY the active layer's heightfield (leaving other layers untouched).
pub fn clear_heightfield(layout: &mut Layout) -> bool {
    let (active_lid, _, editable, _) = active_layer_info(layout);
    if !editable {
        return false;
    }
    if let Some(layers) = layout.layers.as_mut().filter(|ls| !ls.is_empty()) {
        let idx = layers
            .iter()
            .position(|l| Some(l.id.as_str()) == active_lid.as_deref())
            .unwrap_or(0);
        let mut cleared = layers[idx].terrain_field.take().is_some();
        if idx == 0 {
            cleared |= layout.terrain_field.take().is_some();
        }
        return cleared;
    }
    layout.terrain_field.take().is_some()
}

/// Stamps an organic mountain massif or ridge directly into the active layer's heightfield.
pub fn stamp_preset_into_heightfield(layout: &mut Layout, preset: &str, max_h: f32) -> bool {
    let (active_lid, _, editable, _) = active_layer_info(layout);
    if !editable {
        return false;
    }
    let boards: Vec<BaseboardConfig> = board_tools::get_baseboards_slice(layout)
        .into_iter()
        .filter(|b| b.enabled)
        .collect();
    let support_polys = collect_terrain_support_polygons(layout, active_lid.as_deref(), &boards);
    let clip_support = !support_polys.is_empty();
    let (poly_offsets, poly_coords) = pack_support_polygons(&support_polys);
    let (bmin_x, bmin_z, bmax_x, bmax_z) = active_layer_bounds(layout);
    let bw = (bmax_x - bmin_x).max(200.0);
    let bd = (bmax_z - bmin_z).max(200.0);
    let h_peak = max_h.clamp(60.0, MAX_HEIGHT_MM);

    let lobes: Vec<(f64, f64, f64, f32)> = match preset {
        "corner" => vec![
            (bmin_x + bw * 0.16, bmin_z + bd * 0.24, bw.min(bd) * 0.28, h_peak * 0.85),
            (bmin_x + bw * 0.22, bmin_z + bd * 0.16, bw.min(bd) * 0.34, h_peak),
            (bmin_x + bw * 0.34, bmin_z + bd * 0.15, bw.min(bd) * 0.26, h_peak * 0.78),
        ],
        "ridge" => vec![
            (bmin_x + bw * 0.24, bmin_z + bd * 0.18, bd * 0.28, h_peak * 0.82),
            (bmin_x + bw * 0.42, bmin_z + bd * 0.15, bd * 0.32, h_peak * 0.96),
            (bmin_x + bw * 0.58, bmin_z + bd * 0.17, bd * 0.30, h_peak),
            (bmin_x + bw * 0.76, bmin_z + bd * 0.20, bd * 0.26, h_peak * 0.80),
        ],
        _ => vec![(
            (bmin_x + bmax_x) * 0.5,
            (bmin_z + bmax_z) * 0.5,
            bw.min(bd) * 0.26,
            h_peak,
        )],
    };

    let Some(hf) = ensure_active_layer_heightfield(layout) else {
        return false;
    };
    let cols = hf.cols;
    let rows = hf.rows;
    let span_x = (hf.max_x - hf.min_x).max(1.0);
    let span_z = (hf.max_z - hf.min_z).max(1.0);
    let heights = Arc::make_mut(&mut hf.heights);
    let mut changed = false;

    for iz in 0..rows {
        let wz = hf.min_z + (iz as f64 / (rows - 1).max(1) as f64) * span_z;
        for ix in 0..cols {
            let wx = hf.min_x + (ix as f64 / (cols - 1).max(1) as f64) * span_x;
            if clip_support && !is_point_in_any_polygon(wx, wz, &poly_offsets, &poly_coords) {
                continue;
            }
            let noise = ridged_fbm_noise(wx / 210.0, wz / 210.0) as f32;
            let mut lobe_h = 0.0_f32;
            for &(lx, lz, lr, lh) in &lobes {
                let r = lr.max(90.0);
                let d = (wx - lx).hypot(wz - lz) / r;
                if d < 1.0 {
                    let prof = (d * std::f64::consts::FRAC_PI_2).cos().powf(1.35) as f32;
                    let cand = (lh * prof * (0.78 + 0.34 * noise)).clamp(0.0, MAX_HEIGHT_MM);
                    let k = 28.0_f32;
                    let h_diff = (cand - lobe_h).abs();
                    let sm = (k - h_diff).max(0.0) * (1.0 / k);
                    lobe_h = cand.max(lobe_h) + 0.25 * k * sm * sm;
                }
            }
            if lobe_h > 0.5 {
                let idx = iz * cols + ix;
                let cur = heights[idx];
                if lobe_h > cur {
                    heights[idx] = lobe_h.min(MAX_HEIGHT_MM);
                    changed = true;
                }
            }
        }
    }
    if changed {
        hf.revision = hf.revision.wrapping_add(1);
    }
    changed
}

fn mark_buried_tracks_for_hf(
    hf: &TerrainHeightfield,
    target_layer_id: Option<&str>,
    pieces: &mut [TrackPiece],
) -> usize {
    let mut count = 0usize;
    for piece in pieces {
        if let Some(lid) = target_layer_id {
            if piece.layer_id.as_deref().unwrap_or("layer-1") != lid {
                continue;
            }
        }
        let e0 = piece.elevation.map(|e| e.start).unwrap_or(0.0);
        let e1 = piece
            .elevation
            .map(|e| e.end)
            .unwrap_or_else(|| piece.elevation.map(|e| e.start).unwrap_or(0.0));

        let mut buried = false;
        for br in sample_track_branches_world(piece, None) {
            let n = (br.len().saturating_sub(1)).max(1) as f64;
            for (k, pt) in br.iter().enumerate() {
                let rel_el = (e0 + (e1 - e0) * (k as f64 / n)) as f32;
                let surf = sample_heightfield_bilinear(hf, pt.x, pt.y);
                if surf >= rel_el + 42.0 {
                    buried = true;
                    break;
                }
            }
            if buried {
                break;
            }
        }
        if buried {
            if piece.has_tunnel != Some(true) {
                piece.has_tunnel = Some(true);
                piece.has_bridge = Some(false);
                count += 1;
            } else {
                piece.has_bridge = Some(false);
            }
        }
    }
    count
}

/// Scans ONLY active-layer tracks/roads against the active layer's heightfield and automatically
/// sets `has_tunnel = Some(true)` (and clears `has_bridge`) when overburden >= 42 mm.
pub fn apply_auto_tunnel_for_heightfield(layout: &mut Layout) -> usize {
    let (active_lid, _, editable, _) = active_layer_info(layout);
    if !editable {
        return 0;
    }
    let Some(hf) = active_layer_heightfield(layout).cloned() else {
        return 0;
    };
    mark_buried_tracks_for_hf(&hf, active_lid.as_deref(), &mut layout.pieces)
}

/// Scans every visible layer's heightfield against tracks on that layer (used by global Auto-Tunnel).
pub fn apply_auto_tunnel_for_all_layer_heightfields(layout: &mut Layout) -> usize {
    let mut count = 0usize;
    if let Some(layers) = layout.layers.as_ref().filter(|ls| !ls.is_empty()).cloned() {
        for lay in &layers {
            if !lay.visible {
                continue;
            }
            if let Some(ref hf) = lay.terrain_field {
                count += mark_buried_tracks_for_hf(hf, Some(lay.id.as_str()), &mut layout.pieces);
            }
        }
        if let Some(ref hf) = layout.terrain_field {
            count += mark_buried_tracks_for_hf(hf, Some(layers[0].id.as_str()), &mut layout.pieces);
        }
    } else if let Some(ref hf) = layout.terrain_field.clone() {
        count += mark_buried_tracks_for_hf(hf, None, &mut layout.pieces);
    }
    count
}

#[allow(dead_code)]
pub fn default_auto_biome_label() -> u8 {
    BIOME_AUTO
}

#[allow(dead_code)]
pub fn default_grass_biome() -> u8 {
    BIOME_GRASS
}
