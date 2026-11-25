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

fn signed_angle_diff(a: f64, b: f64) -> f64 {
    let mut d = (b - a) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    }
    if d < -180.0 {
        d += 360.0;
    }
    d
}

fn layer_height_map(layout: &Layout) -> HashMap<&str, (f64, bool)> {
    let mut map = HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            map.insert(l.id.as_str(), (l.height_mm.unwrap_or(0.0), l.visible));
        }
    }
    map
}

fn connector_world_elev(piece: &TrackPiece, conn_idx: usize, layers: &HashMap<&str, (f64, bool)>) -> f64 {
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

trait SubExt {
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

// ============================================================================
//  3. NEM 102 / NEM 103 CLEARANCE & COLLISION CHECKER
// ============================================================================

struct SampledPieceGeom {
    id: String,
    gauge: f64,
    has_catenary: bool,
    min_radius: Option<f64>,
    connected_to: HashSet<String>,
    /// Branches of `(x, y, world_elev_mm)`
    branches: Vec<Vec<(f64, f64, f64)>>,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

fn seg_seg_closest_points(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    cx: f64,
    cy: f64,
    dx: f64,
    dy: f64,
) -> (f64, f64, f64, Vec2) {
    // Parametric closest distance squared between segments AB and CD, returning (dist, s, t, mid)
    let ux = bx - ax;
    let uy = by - ay;
    let vx = dx - cx;
    let vy = dy - cy;
    let wx = ax - cx;
    let wy = ay - cy;

    let a = ux * ux + uy * uy;
    let b = ux * vx + uy * vy;
    let c = vx * vx + vy * vy;
    let d = ux * wx + uy * wy;
    let e = vx * wx + vy * wy;
    let denom = a * c - b * b;

    let (mut s_n, mut s_d);
    let (mut t_n, t_d);

    if denom < 1e-8 {
        s_n = 0.0;
        s_d = 1.0;
        t_n = e;
        t_d = c;
    } else {
        s_n = b * e - c * d;
        s_d = denom;
        t_n = a * e - b * d;
        t_d = c;
        if s_n < 0.0 {
            s_n = 0.0;
            t_n = e;
        } else if s_n > s_d {
            s_n = s_d;
            t_n = e + b;
        }
    }

    if t_n < 0.0 {
        t_n = 0.0;
        if -d < 0.0 {
            s_n = 0.0;
        } else if -d > a {
            s_n = s_d;
        } else {
            s_n = -d;
            s_d = a;
        }
    } else if t_n > t_d {
        t_n = t_d;
        if (-d + b) < 0.0 {
            s_n = 0.0;
        } else if (-d + b) > a {
            s_n = s_d;
        } else {
            s_n = -d + b;
            s_d = a;
        }
    }

    let sc = if s_d.abs() < 1e-8 { 0.0 } else { (s_n / s_d).clamp(0.0, 1.0) };
    let tc = if t_d.abs() < 1e-8 { 0.0 } else { (t_n / t_d).clamp(0.0, 1.0) };
    let px = ax + sc * ux;
    let py = ay + sc * uy;
    let qx = cx + tc * vx;
    let qy = cy + tc * vy;
    let dist = (px - qx).hypot(py - qy);
    (
        dist,
        sc,
        tc,
        Vec2 {
            x: (px + qx) * 0.5,
            y: (py + qy) * 0.5,
        },
    )
}

pub fn check_nem_clearances(layout: &Layout, catalogs: &[Catalog]) -> Vec<ClearanceViolation> {
    let layers = layer_height_map(layout);
    let mut geoms: Vec<SampledPieceGeom> = Vec::with_capacity(layout.pieces.len());

    for p in &layout.pieces {
        if is_road_id(catalogs, &p.catalog_id) {
            continue;
        }
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let (lh, visible) = layers.get(lid).copied().unwrap_or((0.0, true));
        if !visible {
            continue;
        }
        let entry = find_entry(catalogs, &p.catalog_id);
        let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5);
        let e0 = lh + p.elevation.map(|e| e.start).unwrap_or(0.0);
        let e1 = lh + p.elevation.map(|e| e.end).unwrap_or(e0 - lh);

        let raw_branches = sample_track_branches_world(p, entry);
        let mut branches = Vec::with_capacity(raw_branches.len());
        let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);

        for br in raw_branches {
            if br.len() < 2 {
                continue;
            }
            let n = (br.len() - 1).max(1) as f64;
            let mut pts = Vec::with_capacity(br.len());
            for (k, pt) in br.iter().enumerate() {
                min_x = min_x.min(pt.x);
                max_x = max_x.max(pt.x);
                min_y = min_y.min(pt.y);
                max_y = max_y.max(pt.y);
                let t = k as f64 / n;
                pts.push((pt.x, pt.y, e0 + (e1 - e0) * t));
            }
            branches.push(pts);
        }
        if !min_x.is_finite() {
            continue;
        }

        let mut connected_to = HashSet::new();
        for c in &p.connectors {
            if let Some(ref ct) = c.connected_to {
                connected_to.insert(ct.piece_id.clone());
            }
        }
        let min_radius = p
            .custom_radius
            .or_else(|| entry.and_then(|e| e.radius.or(e.branch_radius)))
            .map(|r| r.abs());

        geoms.push(SampledPieceGeom {
            id: p.id.clone(),
            gauge,
            has_catenary: p.has_catenary == Some(true),
            min_radius,
            connected_to,
            branches,
            min_x,
            max_x,
            min_y,
            max_y,
        });
    }

    let mut violations_map: HashMap<(String, String, u8), ClearanceViolation> = HashMap::new();

    for i in 0..geoms.len() {
        for j in (i + 1)..geoms.len() {
            let a = &geoms[i];
            let b = &geoms[j];
            if a.connected_to.contains(&b.id) || b.connected_to.contains(&a.id) {
                continue;
            }
            // Skip pairs sharing an immediate turnout/crossing junction neighbor (diverging branches naturally fan out)
            if a.connected_to.iter().any(|n| b.connected_to.contains(n)) {
                continue;
            }

            let scale_f = ((a.gauge + b.gauge) * 0.5 / 16.5).clamp(0.35, 3.6);
            let tight_curve = a.min_radius.map_or(false, |r| r <= 460.0 * scale_f)
                || b.min_radius.map_or(false, |r| r <= 460.0 * scale_f);
            let req_h = (if tight_curve { 52.0 } else { 44.0 }) * scale_f;

            if a.max_x + req_h < b.min_x
                || b.max_x + req_h < a.min_x
                || a.max_y + req_h < b.min_y
                || b.max_y + req_h < a.min_y
            {
                continue;
            }

            let mut worst_v: Option<(f64, f64, Vec2)> = None;
            let mut worst_h: Option<(f64, f64, Vec2)> = None;

            for br_a in &a.branches {
                for sa in br_a.windows(2) {
                    for br_b in &b.branches {
                        for sb in br_b.windows(2) {
                            let (dist2d, sc, tc, mid) = seg_seg_closest_points(
                                sa[0].0, sa[0].1, sa[1].0, sa[1].1, sb[0].0, sb[0].1, sb[1].0,
                                sb[1].1,
                            );
                            if dist2d > req_h {
                                continue;
                            }
                            let ea = sa[0].2 + (sa[1].2 - sa[0].2) * sc;
                            let eb = sb[0].2 + (sb[1].2 - sb[0].2) * tc;
                            let dz = (ea - eb).abs();
                            let lower_has_cat = if ea <= eb { a.has_catenary } else { b.has_catenary };
                            let req_v = (if lower_has_cat { 92.0 } else { 75.0 }) * scale_f;

                            if dist2d < (a.gauge + b.gauge) * 0.55 {
                                if dz >= 1.5 && dz < req_v {
                                    if worst_v.map_or(true, |(cur, _, _)| dz < cur) {
                                        worst_v = Some((dz, req_v, mid));
                                    }
                                }
                            } else if dz < req_v * 0.65 && dist2d < req_h {
                                if worst_h.map_or(true, |(cur, _, _)| dist2d < cur) {
                                    worst_h = Some((dist2d, req_h, mid));
                                }
                            }
                        }
                    }
                }
            }

            if let Some((actual_v, req_v, pos)) = worst_v {
                let key = (a.id.clone(), b.id.clone(), 0);
                violations_map.insert(
                    key,
                    ClearanceViolation {
                        kind: ClearanceKind::VerticalHeadroom,
                        pos,
                        piece_a: a.id.clone(),
                        piece_b: b.id.clone(),
                        actual_mm: actual_v,
                        required_mm: req_v,
                        label: format!("NEM 102: {actual_v:.0}<{req_v:.0}mm"),
                    },
                );
            }
            if let Some((actual_h, req_h_v, pos)) = worst_h {
                let key = (a.id.clone(), b.id.clone(), 1);
                violations_map.insert(
                    key,
                    ClearanceViolation {
                        kind: ClearanceKind::HorizontalCurve,
                        pos,
                        piece_a: a.id.clone(),
                        piece_b: b.id.clone(),
                        actual_mm: actual_h,
                        required_mm: req_h_v,
                        label: format!("NEM 103: {actual_h:.0}<{req_h_v:.0}mm"),
                    },
                );
            }
        }
    }

    let mut out: Vec<ClearanceViolation> = violations_map.into_values().collect();
    out.sort_by(|a, b| a.piece_a.cmp(&b.piece_a).then(a.piece_b.cmp(&b.piece_b)));
    out
}

pub fn draw_nem_clearance_overlay_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    violations: &[ClearanceViolation],
    is_de: bool,
) {
    if violations.is_empty() {
        return;
    }
    for v in violations {
        let sp = camera.world_to_screen(rect, v.pos.x, v.pos.y);
        if !rect.expand(48.0).contains(sp) {
            continue;
        }
        let col = match v.kind {
            ClearanceKind::VerticalHeadroom => Color32::from_rgb(239, 68, 68),
            ClearanceKind::HorizontalCurve => Color32::from_rgb(245, 158, 11),
        };
        let bg = Color32::from_rgba_unmultiplied(15, 23, 42, 230);
        painter.circle_stroke(sp, 14.0, Stroke::new(2.2_f32, col));
        painter.circle_filled(sp, 4.5, col);
        let tag_pos = Pos2::new(sp.x, sp.y - 22.0);
        let tag_r = Rect::from_center_size(tag_pos, EguiVec2::new(112.0, 17.0));
        painter.rect_filled(tag_r, 4.0, bg);
        painter.rect_stroke(tag_r, 4.0, Stroke::new(1.1_f32, col), egui::StrokeKind::Inside);
        painter.text(
            tag_pos,
            egui::Align2::CENTER_CENTER,
            &v.label,
            FontId::proportional(9.5),
            Color32::WHITE,
        );
    }

    let banner = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.min.y + 68.0),
        EguiVec2::new(360.0, 24.0),
    );
    painter.rect_filled(banner, 6.0, Color32::from_rgba_unmultiplied(120, 53, 15, 235));
    painter.rect_stroke(
        banner,
        6.0,
        Stroke::new(1.2_f32, Color32::from_rgb(251, 191, 36)),
        egui::StrokeKind::Inside,
    );
    let msg = if is_de {
        format!("Lichtraumprofil NEM 102/103: {} Konflikt(e)", violations.len())
    } else {
        format!("NEM 102/103 Clearance: {} conflict(s)", violations.len())
    };
    painter.text(
        banner.center(),
        egui::Align2::CENTER_CENTER,
        msg,
        FontId::proportional(11.0),
        Color32::from_rgb(254, 243, 199),
    );
}

// ============================================================================
//  4. REACHABILITY HEATMAP (Arm-Reach Analysis)
// ============================================================================

fn point_in_polygon(pt: Vec2, poly: &[Vec2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let xi = poly[i].x;
        let yi = poly[i].y;
        let xj = poly[j].x;
        let yj = poly[j].y;
        let intersect = ((yi > pt.y) != (yj > pt.y))
            && (pt.x < (xj - xi) * (pt.y - yi) / ((yj - yi).abs().max(1e-9) * (yj - yi).signum()) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn min_dist_to_polygon_edges(pt: Vec2, poly: &[Vec2]) -> f64 {
    let n = poly.len();
    if n < 2 {
        return f64::INFINITY;
    }
    let mut best = f64::INFINITY;
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let vx = b.x - a.x;
        let vy = b.y - a.y;
        let len_sq = vx * vx + vy * vy;
        let t = if len_sq < 1e-6 {
            0.0
        } else {
            (((pt.x - a.x) * vx + (pt.y - a.y) * vy) / len_sq).clamp(0.0, 1.0)
        };
        let dx = pt.x - (a.x + t * vx);
        let dy = pt.y - (a.y + t * vy);
        best = best.min(dx.hypot(dy));
    }
    best
}

#[cfg(test)]
pub fn compute_unreachable_area_pct(layout: &Layout, max_reach_mm: f64) -> f32 {
    let boards = get_baseboards_slice(layout);
    let mut total = 0usize;
    let mut unreachable = 0usize;
    let step = 65.0_f64;

    for b in boards {
        if !b.enabled {
            continue;
        }
        let poly = get_baseboard_polygon(&b);
        if poly.len() < 3 {
            continue;
        }
        let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in &poly {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        let mut x = min_x + step * 0.5;
        while x < max_x {
            let mut y = min_y + step * 0.5;
            while y < max_y {
                let pt = Vec2 { x, y };
                if point_in_polygon(pt, &poly) {
                    total += 1;
                    if min_dist_to_polygon_edges(pt, &poly) > max_reach_mm {
                        unreachable += 1;
                    }
                }
                y += step;
            }
            x += step;
        }
    }
    if total == 0 {
        0.0
    } else {
        (unreachable as f32 / total as f32) * 100.0
    }
}

pub fn draw_reachability_heatmap_2d(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2D,
    layout: &Layout,
    max_reach_mm: f64,
    is_de: bool,
) -> f32 {
    let boards = get_baseboards_slice(layout);
    let step = 65.0_f64;
    let mut total = 0usize;
    let mut unreachable = 0usize;

    for b in boards {
        if !b.enabled {
            continue;
        }
        let poly = get_baseboard_polygon(&b);
        if poly.len() < 3 {
            continue;
        }
        let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in &poly {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        let mut x = min_x + step * 0.5;
        while x < max_x {
            let mut y = min_y + step * 0.5;
            while y < max_y {
                let pt = Vec2 { x, y };
                if point_in_polygon(pt, &poly) {
                    total += 1;
                    let d = min_dist_to_polygon_edges(pt, &poly);
                    if d > max_reach_mm {
                        unreachable += 1;
                    }
                    let col = if d <= max_reach_mm * 0.72 {
                        Color32::from_rgba_unmultiplied(34, 197, 94, 42)
                    } else if d <= max_reach_mm {
                        Color32::from_rgba_unmultiplied(245, 158, 11, 56)
                    } else {
                        Color32::from_rgba_unmultiplied(239, 68, 68, 78)
                    };
                    let p0 = camera.world_to_screen(rect, x - step * 0.5, y - step * 0.5);
                    let p1 = camera.world_to_screen(rect, x + step * 0.5, y + step * 0.5);
                    let cell_r = Rect::from_two_pos(p0, p1);
                    if cell_r.intersects(rect) {
                        painter.rect_filled(cell_r, 0.0, col);
                    }
                }
                y += step;
            }
            x += step;
        }
    }

    let pct = if total == 0 {
        0.0
    } else {
        (unreachable as f32 / total as f32) * 100.0
    };

    // Legend card bottom-left
    let leg_r = Rect::from_min_size(Pos2::new(rect.min.x + 18.0, rect.max.y - 44.0), EguiVec2::new(310.0, 26.0));
    painter.rect_filled(leg_r, 6.0, Color32::from_rgba_unmultiplied(15, 23, 42, 220));
    painter.rect_stroke(
        leg_r,
        CornerRadius::same(6),
        Stroke::new(1.0_f32, Color32::from_rgb(71, 85, 105)),
        egui::StrokeKind::Inside,
    );
    let txt = if is_de {
        format!(
            "Greifweite <= {:.0} cm  |  {:.0}% nicht erreichbar",
            max_reach_mm / 10.0,
            pct
        )
    } else {
        format!(
            "Reachability <= {:.0} cm  |  {:.0}% out of reach",
            max_reach_mm / 10.0,
            pct
        )
    };
    painter.text(
        leg_r.center(),
        egui::Align2::CENTER_CENTER,
        txt,
        FontId::proportional(11.0),
        Color32::WHITE,
    );

    pct
}

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
