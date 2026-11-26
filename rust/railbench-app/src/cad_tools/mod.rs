use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, Vec2 as EguiVec2};
use railbench_core::geometry::{norm_angle_deg, rotate};
use railbench_core::snapping::{piece_connector_height, update_layout_connections};
use railbench_core::types::{Catalog, CatalogEntry, Elevation, Layout, TrackPiece, Transform, Vec2};

use crate::board_tools::{get_baseboard_polygon, get_baseboards_slice};
use crate::catalog::{
    create_piece, disconnect_cross_type_connections, find_entry, is_road_id, local_connectors,
    snap_flex_endpoint,
};
use crate::export::sample_track_branches_world;

use crate::canvas2d::Camera2D;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearanceKind {
    VerticalHeadroom,
    HorizontalCurve,
}

#[derive(Debug, Clone)]
pub struct ClearanceViolation {
    pub kind: ClearanceKind,
    pub pos: Vec2,
    pub piece_a: String,
    pub piece_b: String,
    pub actual_mm: f64,
    pub required_mm: f64,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct CadToolsState {
    pub parallel_spacing_mm: f64,
    pub parallel_side: f64,
    pub nem_check_enabled: bool,
    pub nem_cache_version: u64,
    pub nem_violations: Vec<ClearanceViolation>,
    pub nem_focus_idx: usize,
    pub reachability_enabled: bool,
    pub max_reach_mm: f64,
    pub last_unreachable_pct: f32,
}

impl Default for CadToolsState {
    fn default() -> Self {
        Self {
            parallel_spacing_mm: 64.3,
            parallel_side: 1.0,
            nem_check_enabled: false,
            nem_cache_version: 0,
            nem_violations: Vec::new(),
            nem_focus_idx: 0,
            reachability_enabled: false,
            max_reach_mm: 800.0,
            last_unreachable_pct: 0.0,
        }
    }
}

impl CadToolsState {
    pub fn default_spacing_for_catalog(scale: &str, catalog_id: &str) -> f64 {
        let lower = catalog_id.to_ascii_lowercase();
        if lower.contains("maerklin-c") || lower.contains("trix-c") {
            return 64.3;
        }
        if lower.contains("maerklin-k") {
            return 64.6;
        }
        if lower.contains("piko-a") {
            return 61.88;
        }
        if lower.contains("roco-line") {
            return 61.6;
        }
        match scale.to_ascii_uppercase().as_str() {
            "TT" => 43.0,
            "N" => 33.6,
            "Z" => 25.0,
            "0" => 105.0,
            "1" => 145.0,
            "G" => 160.0,
            _ => 64.3,
        }
    }
}

pub(super) fn signed_angle_diff(a: f64, b: f64) -> f64 {
    let mut d = (b - a) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    }
    if d < -180.0 {
        d += 360.0;
    }
    d
}

pub(super) fn layer_height_map(layout: &Layout) -> HashMap<&str, (f64, bool)> {
    let mut map = HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            map.insert(l.id.as_str(), (l.height_mm.unwrap_or(0.0), l.visible));
        }
    }
    map
}

pub(super) fn connector_world_elev(piece: &TrackPiece, conn_idx: usize, layers: &HashMap<&str, (f64, bool)>) -> f64 {
    let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
    let lh = layers.get(lid).map(|v| v.0).unwrap_or(0.0);
    piece_connector_height(piece, conn_idx, lh)
}

fn find_preferred_rail_catalog<'a>(
    catalogs: &'a [Catalog],
    piece_cat_id: &str,
    active_catalog_idx: usize,
) -> Option<&'a Catalog> {
    if let Some(cat) = catalogs.iter().find(|c| {
        c.is_road != Some(true) && c.entries.iter().any(|e| e.id == piece_cat_id)
    }) {
        return Some(cat);
    }
    if let Some(cat) = catalogs.get(active_catalog_idx) {
        if cat.is_road != Some(true) {
            return Some(cat);
        }
    }
    catalogs.iter().find(|c| c.is_road != Some(true))
}

fn snap_transform_for_entry(entry: &CatalogEntry, target_pos: Vec2, target_angle: f64, conn_idx: usize) -> Option<Transform> {
    let locals = local_connectors(entry);
    let lc = locals.get(conn_idx)?;
    let rot = norm_angle_deg(target_angle + 180.0 - lc.angle);
    let ro = rotate(Vec2 { x: lc.x, y: lc.y }, rot);
    Some(Transform {
        x: target_pos.x - ro.x,
        y: target_pos.y - ro.y,
        rotation: rot,
    })
}

fn connector_world_pose(entry: &CatalogEntry, tr: Transform, conn_idx: usize) -> Option<(Vec2, f64)> {
    let locals = local_connectors(entry);
    let lc = locals.get(conn_idx)?;
    let ro = rotate(Vec2 { x: lc.x, y: lc.y }, tr.rotation);
    Some((
        Vec2 {
            x: tr.x + ro.x,
            y: tr.y + ro.y,
        },
        norm_angle_deg(tr.rotation + lc.angle),
    ))
}

// ============================================================================
//  1. GAP CLOSER ("Gleisluecke schliessen")
// ============================================================================

/// Bridges an open gap between two selected tracks (or between a single selected track's open
/// connector and the closest opposing open connector, or closest global pair if nothing is selected).
/// Uses a 3-tier solver: exact 1-piece catalog match -> 2-piece combo -> smooth flex-track spline.
pub fn close_gap_between_selected(
    layout: &mut Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    next_seq: &mut u64,
    active_catalog_idx: usize,
) -> Result<(Vec<String>, String), String> {
    let layers = layer_height_map(layout);

    // Collect open non-road connectors
    let mut open_conns: Vec<(usize, usize, Vec2, f64, f64, bool)> = Vec::new();
    for (pi, p) in layout.pieces.iter().enumerate() {
        if is_road_id(catalogs, &p.catalog_id) {
            continue;
        }
        let is_sel = selected.contains(&p.id);
        for (ci, c) in p.connectors.iter().enumerate() {
            if c.connected_to.is_none() {
                let elev = connector_world_elev(p, ci, &layers);
                open_conns.push((pi, ci, Vec2 { x: c.x, y: c.y }, c.angle, elev, is_sel));
            }
        }
    }

    if open_conns.len() < 2 {
        return Err("Need at least two open track connectors to close a gap.".into());
    }

    let selected_count = selected
        .iter()
        .filter(|sid| layout.pieces.iter().any(|p| &p.id == *sid && !is_road_id(catalogs, &p.catalog_id)))
        .count();

    let mut best_pair: Option<(f64, usize, usize)> = None;
    for i in 0..open_conns.len() {
        for j in (i + 1)..open_conns.len() {
            let a = &open_conns[i];
            let b = &open_conns[j];
            if a.0 == b.0 {
                continue;
            }
            if selected_count >= 2 && !(a.5 && b.5) {
                continue;
            }
            if selected_count == 1 && !(a.5 || b.5) {
                continue;
            }
            let dist = (a.2.x - b.2.x).hypot(a.2.y - b.2.y);
            if !(18.0..=950.0).contains(&dist) {
                continue;
            }
            if (a.4 - b.4).abs() > 80.0 {
                continue;
            }
            if best_pair.map_or(true, |(bd, _, _)| dist < bd) {
                best_pair = Some((dist, i, j));
            }
        }
    }

    let Some((gap_dist, idx_a, idx_b)) = best_pair else {
        return Err("No compatible open track connector pair found within 18..950 mm.".into());
    };

    let (pi_a, _, pa, ang_a, elev_a, _) = open_conns[idx_a];
    let (_, _, pb, ang_b, elev_b, _) = open_conns[idx_b];
    let target_layer_id = layout.pieces[pi_a].layer_id.clone();
    let target_lh = layers
        .get(target_layer_id.as_deref().unwrap_or("layer-1"))
        .map(|v| v.0)
        .unwrap_or(0.0);

    let pref_cat = find_preferred_rail_catalog(
        catalogs,
        &layout.pieces[pi_a].catalog_id,
        active_catalog_idx,
    )
    .ok_or_else(|| "No rail catalog available for gap closing.".to_string())?;

    let mut candidates: Vec<&CatalogEntry> = pref_cat
        .entries
        .iter()
        .filter(|e| {
            e.is_road != Some(true)
                && matches!(e.kind.as_str(), "straight" | "curve")
                && local_connectors(e).len() == 2
        })
        .collect();
    // Deduplicate geometry signatures for fast 2-piece search
    candidates.truncate(36);

    let desired_end_ang = norm_angle_deg(ang_b + 180.0);

    // Tier 1: Exact 1-piece match from catalog
    let mut best_1: Option<(f64, &CatalogEntry, Transform)> = None;
    for &e in &candidates {
        for start_ci in [0usize, 1usize] {
            let end_ci = 1 - start_ci;
            if let Some(tr) = snap_transform_for_entry(e, pa, ang_a, start_ci) {
                if let Some((ep, eang)) = connector_world_pose(e, tr, end_ci) {
                    let pos_err = ep.x.sub(pb.x).hypot(ep.y - pb.y);
                    let ang_err = signed_angle_diff(eang, desired_end_ang).abs();
                    if pos_err <= 3.2 && ang_err <= 4.5 {
                        let score = pos_err + ang_err * 0.6;
                        if best_1.map_or(true, |(bs, _, _)| score < bs) {
                            best_1 = Some((score, e, tr));
                        }
                    }
                }
            }
        }
    }

    if let Some((_, entry, tr)) = best_1 {
        let id = format!("piece-{}", *next_seq);
        *next_seq += 1;
        let mut piece = create_piece(id.clone(), entry, tr, target_layer_id);
        let rel_a = elev_a - target_lh;
        let rel_b = elev_b - target_lh;
        if rel_a.abs() > 0.2 || rel_b.abs() > 0.2 {
            piece.elevation = Some(Elevation {
                start: rel_a,
                end: rel_b,
            });
        }
        layout.pieces.push(piece);
        update_layout_connections(layout, 4.0, 35.0, 1.0);
        disconnect_cross_type_connections(layout, catalogs);
        return Ok((
            vec![id],
            format!("Gap closed with 1x {} ({:.1} mm)", entry.id, gap_dist),
        ));
    }

    // Tier 2: Exact 2-piece match from catalog (straight+curve, curve+curve, or 2 straights)
    let mut best_2: Option<(f64, &CatalogEntry, Transform, &CatalogEntry, Transform)> = None;
    for &e1 in &candidates {
        for c1_start in [0usize, 1usize] {
            let c1_end = 1 - c1_start;
            let Some(tr1) = snap_transform_for_entry(e1, pa, ang_a, c1_start) else {
                continue;
            };
            let Some((mid_pos, mid_ang)) = connector_world_pose(e1, tr1, c1_end) else {
                continue;
            };
            for &e2 in &candidates {
                for c2_start in [0usize, 1usize] {
                    let c2_end = 1 - c2_start;
                    let Some(tr2) = snap_transform_for_entry(e2, mid_pos, mid_ang, c2_start) else {
                        continue;
                    };
                    if let Some((ep, eang)) = connector_world_pose(e2, tr2, c2_end) {
                        let pos_err = ep.x.sub(pb.x).hypot(ep.y - pb.y);
                        let ang_err = signed_angle_diff(eang, desired_end_ang).abs();
                        if pos_err <= 3.5 && ang_err <= 5.5 {
                            let score = pos_err + ang_err * 0.6;
                            if best_2.map_or(true, |(bs, _, _, _, _)| score < bs) {
                                best_2 = Some((score, e1, tr1, e2, tr2));
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some((_, e1, tr1, e2, tr2)) = best_2 {
        let id1 = format!("piece-{}", *next_seq);
        *next_seq += 1;
        let id2 = format!("piece-{}", *next_seq);
        *next_seq += 1;
        let mut p1 = create_piece(id1.clone(), e1, tr1, target_layer_id.clone());
        let mut p2 = create_piece(id2.clone(), e2, tr2, target_layer_id);
        let rel_a = elev_a - target_lh;
        let rel_b = elev_b - target_lh;
        let rel_m = (rel_a + rel_b) * 0.5;
        if rel_a.abs() > 0.2 || rel_b.abs() > 0.2 {
            p1.elevation = Some(Elevation {
                start: rel_a,
                end: rel_m,
            });
            p2.elevation = Some(Elevation {
                start: rel_m,
                end: rel_b,
            });
        }
        layout.pieces.push(p1);
        layout.pieces.push(p2);
        update_layout_connections(layout, 4.0, 35.0, 1.0);
        disconnect_cross_type_connections(layout, catalogs);
        return Ok((
            vec![id1, id2],
            format!("Gap closed with {} + {}", e1.id, e2.id),
        ));
    }

    // Tier 3: Smooth flex-track bridge
    let flex_entry = pref_cat
        .entries
        .iter()
        .find(|e| e.kind == "flex" && e.is_road != Some(true))
        .or_else(|| {
            catalogs
                .iter()
                .filter(|c| c.is_road != Some(true))
                .flat_map(|c| c.entries.iter())
                .find(|e| e.kind == "flex")
        });

    let Some(flex_e) = flex_entry else {
        return Err("No matching 1- or 2-piece geometry found and catalog has no Flex Track.".into());
    };

    let tr0 = snap_transform_for_entry(flex_e, pa, ang_a, 0)
        .ok_or_else(|| "Flex track entry has invalid connectors.".to_string())?;
    let id = format!("piece-{}", *next_seq);
    *next_seq += 1;
    let mut flex_piece = create_piece(id.clone(), flex_e, tr0, target_layer_id);
    if !snap_flex_endpoint(&mut flex_piece, catalogs, 1, pb, ang_b) {
        return Err(format!(
            "Gap ({:.0} mm) exceeds flex track bounds or curvature limit.",
            gap_dist
        ));
    }
    let rel_a = elev_a - target_lh;
    let rel_b = elev_b - target_lh;
    if rel_a.abs() > 0.2 || rel_b.abs() > 0.2 {
        flex_piece.elevation = Some(Elevation {
            start: rel_a,
            end: rel_b,
        });
    }
    let len_str = flex_piece
        .custom_length
        .map(|l| format!("{l:.0} mm"))
        .unwrap_or_else(|| format!("{gap_dist:.0} mm"));
    layout.pieces.push(flex_piece);
    update_layout_connections(layout, 4.0, 35.0, 1.0);
    disconnect_cross_type_connections(layout, catalogs);
    Ok((
        vec![id],
        format!("Gap bridged with Flex Track {} ({len_str})", flex_e.id),
    ))
}

pub(super) trait SubExt {
    fn sub(self, rhs: f64) -> f64;
}
impl SubExt for f64 {
    fn sub(self, rhs: f64) -> f64 {
        self - rhs
    }
}

// ============================================================================
//  2. PARALLEL TRACK GENERATOR ("Parallelgleis")
// ============================================================================

/// Generates parallel tracks at `spacing_mm` on `side` (`+1.0` right / `-1.0` left of heading)
/// for all currently selected non-road pieces, matching catalog radii automatically for curves.
pub fn generate_parallel_tracks(
    layout: &mut Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    spacing_mm: f64,
    side: f64,
    next_seq: &mut u64,
) -> Result<Vec<String>, String> {
    if spacing_mm.abs() < 15.0 {
        return Err("Parallel track spacing must be at least 15 mm.".into());
    }
    let side_sign = if side < 0.0 { -1.0 } else { 1.0 };
    let selected_pieces: Vec<TrackPiece> = layout
        .pieces
        .iter()
        .filter(|p| selected.contains(&p.id) && !is_road_id(catalogs, &p.catalog_id))
        .cloned()
        .collect();

    if selected_pieces.is_empty() {
        return Err("Select one or more track pieces first.".into());
    }

    let mut created_ids = Vec::new();

    for src in selected_pieces {
        let Some(orig_entry) = find_entry(catalogs, &src.catalog_id) else {
            continue;
        };
        let owner_cat = find_preferred_rail_catalog(catalogs, &src.catalog_id, 0);

        match orig_entry.kind.as_str() {
            "curve" => {
                let r = src.custom_radius.or(orig_entry.radius).unwrap_or(360.0);
                let ang = src.custom_angle.or(orig_entry.angle).unwrap_or(30.0);
                let dir = if ang < 0.0 { -1.0 } else { 1.0 };
                let r_target = r - dir * side_sign * spacing_mm;
                if r_target < 85.0 {
                    continue;
                }

                // World center of original curve circle
                let c_off = rotate(Vec2 { x: 0.0, y: dir * r }, src.transform.rotation);
                let center_w = Vec2 {
                    x: src.transform.x + c_off.x,
                    y: src.transform.y + c_off.y,
                };

                // Look for a matching catalog curve in the same system within 18 mm of r_target
                let matched_entry = owner_cat.and_then(|cat| {
                    cat.entries
                        .iter()
                        .filter(|e| e.kind == "curve" && e.is_road != Some(true))
                        .filter_map(|e| {
                            let er = e.radius?;
                            let ea = e.angle?;
                            if ea.signum() != ang.signum() && (ea + ang).abs() > 1e-3 {
                                return None;
                            }
                            if (ea.abs() - ang.abs()).abs() > 2.5 {
                                return None;
                            }
                            let diff = (er - r_target).abs();
                            if diff <= 18.0 {
                                Some((diff, e, er))
                            } else {
                                None
                            }
                        })
                        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                });

                let (use_entry, r_use, custom_rad) = if let Some((_, me, er)) = matched_entry {
                    (me.clone(), er, None)
                } else {
                    let mut clone_e = orig_entry.clone();
                    clone_e.radius = Some(r_target);
                    (clone_e, r_target, Some(r_target))
                };

                let new_c_off = rotate(Vec2 { x: 0.0, y: dir * r_use }, src.transform.rotation);
                let new_tr = Transform {
                    x: center_w.x - new_c_off.x,
                    y: center_w.y - new_c_off.y,
                    rotation: src.transform.rotation,
                };
                let new_id = format!("piece-{}", *next_seq);
                *next_seq += 1;
                let mut np = create_piece(new_id.clone(), &use_entry, new_tr, src.layer_id.clone());
                if custom_rad.is_some() {
                    np.custom_radius = custom_rad;
                    np.custom_angle = Some(ang);
                }
                np.elevation = src.elevation;
                np.has_catenary = src.has_catenary;
                layout.pieces.push(np);
                created_ids.push(new_id);
            }
            "flex" if src.connectors.len() >= 2 => {
                let c0 = &src.connectors[0];
                let c1 = &src.connectors[1];
                let h0 = norm_angle_deg(c0.angle + 180.0);
                let h1 = c1.angle;
                let n0 = rotate(Vec2 { x: 0.0, y: side_sign * spacing_mm }, h0);
                let n1 = rotate(Vec2 { x: 0.0, y: side_sign * spacing_mm }, h1);
                let tr_new = Transform {
                    x: c0.x + n0.x,
                    y: c0.y + n0.y,
                    rotation: h0,
                };
                let target1 = Vec2 {
                    x: c1.x + n1.x,
                    y: c1.y + n1.y,
                };
                let target1_conn_ang = norm_angle_deg(h1 + 180.0);
                let new_id = format!("piece-{}", *next_seq);
                *next_seq += 1;
                let mut np = create_piece(new_id.clone(), orig_entry, tr_new, src.layer_id.clone());
                if snap_flex_endpoint(&mut np, catalogs, 1, target1, target1_conn_ang) {
                    np.elevation = src.elevation;
                    np.has_catenary = src.has_catenary;
                    layout.pieces.push(np);
                    created_ids.push(new_id);
                }
            }
            _ => {
                // Straight, turnout, or crossing: offset perpendicularly to piece rotation
                let n = rotate(Vec2 { x: 0.0, y: side_sign * spacing_mm }, src.transform.rotation);
                let new_tr = Transform {
                    x: src.transform.x + n.x,
                    y: src.transform.y + n.y,
                    rotation: src.transform.rotation,
                };
                let new_id = format!("piece-{}", *next_seq);
                *next_seq += 1;
                let mut np = create_piece(new_id.clone(), orig_entry, new_tr, src.layer_id.clone());
                np.elevation = src.elevation;
                np.has_catenary = src.has_catenary;
                layout.pieces.push(np);
                created_ids.push(new_id);
            }
        }
    }

    if created_ids.is_empty() {
        return Err("No valid parallel tracks could be generated at this spacing.".into());
    }
    update_layout_connections(layout, 4.0, 35.0, 1.0);
    disconnect_cross_type_connections(layout, catalogs);
    Ok(created_ids)
}


mod analysis;
pub use analysis::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, find_entry};

    fn h0_layout_with_board(width: f64, depth: f64) -> Layout {
        let mut l = crate::make_empty_layout();
        l.name = "CadTest".into();
        if let Some(bb) = l.baseboards.as_mut().and_then(|bs| bs.first_mut()) {
            bb.width = width;
            bb.depth = depth;
        }
        l
    }

    #[test]
    fn gap_closer_bridges_straight_gap_with_exact_catalog_piece() {
        let catalogs = builtin_catalogs();
        let e24188 = find_entry(&catalogs, "24188").unwrap();
        let mut layout = h0_layout_with_board(2000.0, 1000.0);
        // Leave exact 188.3 mm gap between p0 [0..188.3] and p1 [376.6..564.9]
        let p0 = create_piece("p0".into(), e24188, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        let p1 = create_piece("p1".into(), e24188, Transform { x: 376.6, y: 0.0, rotation: 0.0 }, None);
        layout.pieces = vec![p0, p1];
        let mut sel = HashSet::from(["p0".into(), "p1".into()]);
        let mut seq = 10u64;

        let (added, msg) = close_gap_between_selected(&mut layout, &catalogs, &sel, &mut seq, 0).unwrap();
        assert_eq!(added.len(), 1);
        assert!(msg.contains("24188"), "expected exact 24188 match, got: {msg}");
        assert_eq!(layout.pieces.len(), 3);
        let bridge = layout.pieces.iter().find(|p| p.id == added[0]).unwrap();
        assert!(bridge.connectors.iter().all(|c| c.connected_to.is_some()));
        sel.clear();
    }

    #[test]
    fn gap_closer_bridges_offset_gap_with_flex_track() {
        let catalogs = builtin_catalogs();
        // Piko A-Gleis has flex track 55209 + 55200 straights
        let e_str = find_entry(&catalogs, "55200").unwrap();
        let cat_idx = catalogs.iter().position(|c| c.entries.iter().any(|e| e.id == "55200")).unwrap();
        let mut layout = h0_layout_with_board(2000.0, 1000.0);
        let p0 = create_piece("p0".into(), e_str, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        // Lateral + axial offset that no standard single straight/curve bridges
        let p1 = create_piece("p1".into(), e_str, Transform { x: 520.0, y: 28.0, rotation: 0.0 }, None);
        layout.pieces = vec![p0, p1];
        let sel = HashSet::from(["p0".into(), "p1".into()]);
        let mut seq = 10u64;

        let (added, _msg) = close_gap_between_selected(&mut layout, &catalogs, &sel, &mut seq, cat_idx).unwrap();
        assert_eq!(added.len(), 1);
        let bridge = layout.pieces.iter().find(|p| p.id == added[0]).unwrap();
        assert!(bridge.connectors.iter().all(|c| c.connected_to.is_some()));
    }

    #[test]
    fn parallel_track_generator_creates_parallel_straight_and_concentric_curve() {
        let catalogs = builtin_catalogs();
        let e24188 = find_entry(&catalogs, "24188").unwrap();
        let e24130 = find_entry(&catalogs, "24130").unwrap();
        let mut layout = h0_layout_with_board(2000.0, 1000.0);
        let p0 = create_piece("p0".into(), e24188, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        let p1 = create_piece("p1".into(), e24130, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, None);
        layout.pieces = vec![p0, p1];
        let sel = HashSet::from(["p0".into(), "p1".into()]);
        let mut seq = 20u64;

        let created = generate_parallel_tracks(&mut layout, &catalogs, &sel, 64.3, -1.0, &mut seq).unwrap();
        assert_eq!(created.len(), 2);
        let par_str = layout.pieces.iter().find(|p| p.id == created[0]).unwrap();
        let par_crv = layout.pieces.iter().find(|p| p.id == created[1]).unwrap();
        assert!((par_str.transform.y - (-64.3)).abs() < 0.01);
        // R1 (360mm) offset outward (-1) snaps to Maerklin R2 (437.5mm -> wait: +64.3 = 424.3 matches R2 437.5 within 18mm!)
        assert_eq!(par_crv.catalog_id, "24230");
    }

    #[test]
    fn nem_clearance_detects_low_bridge_crossing_and_tight_parallel_tracks() {
        let catalogs = builtin_catalogs();
        let e24188 = find_entry(&catalogs, "24188").unwrap();
        let mut layout = h0_layout_with_board(2000.0, 1000.0);
        let mut p_low = create_piece("low".into(), e24188, Transform { x: 0.0, y: 100.0, rotation: 0.0 }, None);
        p_low.elevation = Some(Elevation { start: 0.0, end: 0.0 });
        let mut p_high = create_piece("high".into(), e24188, Transform { x: 94.0, y: 0.0, rotation: 90.0 }, None);
        // Only 42 mm vertical separation -> violates NEM 102 (needs 75 mm)
        p_high.elevation = Some(Elevation { start: 42.0, end: 42.0 });
        // Parallel track only 32 mm away -> violates NEM 103 (needs 44 mm)
        let p_close = create_piece("close".into(), e24188, Transform { x: 0.0, y: 132.0, rotation: 0.0 }, None);
        layout.pieces = vec![p_low, p_high, p_close];

        let violations = check_nem_clearances(&layout, &catalogs);
        assert!(
            violations.iter().any(|v| v.kind == ClearanceKind::VerticalHeadroom),
            "expected NEM 102 vertical headroom violation, got {violations:?}"
        );
        assert!(
            violations.iter().any(|v| v.kind == ClearanceKind::HorizontalCurve),
            "expected NEM 103 horizontal clearance violation, got {violations:?}"
        );
    }

    #[test]
    fn reachability_heatmap_detects_unreachable_center_on_deep_table() {
        // Shallow table (1000 mm deep, max dist from edge 500 mm) -> 0% out of reach at 800 mm
        let shallow = h0_layout_with_board(2000.0, 1000.0);
        assert!(compute_unreachable_area_pct(&shallow, 800.0) < 0.01);

        // Very deep island table (2400 x 2400 mm, center is 1200 mm from edge) -> > 10% unreachable at 800 mm
        let deep = h0_layout_with_board(2400.0, 2400.0);
        let pct = compute_unreachable_area_pct(&deep, 800.0);
        assert!(pct > 10.0 && pct < 30.0, "unexpected unreachable pct {pct}");
    }
}
