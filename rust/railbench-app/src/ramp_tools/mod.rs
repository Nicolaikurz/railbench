use std::collections::{HashMap, HashSet, VecDeque};

use railbench_core::geometry::{deg2rad, norm_angle_deg, rotate};
use railbench_core::ramp::compute_ramp_chain_elevations;
use railbench_core::snapping::update_layout_connections;
use railbench_core::types::{Catalog, CatalogEntry, Elevation, Layout, TrackPiece, Transform, Vec2};

use crate::catalog::{create_piece, disconnect_cross_type_connections, find_entry, local_connectors};

#[derive(Debug, Clone, Default)]
pub struct RampSelectionState {
    pub start_piece: Option<String>,
    pub end_piece: Option<String>,
    /// `Some(true)` = clicking a track picks Start (A); `Some(false)` = clicking a track picks End (B).
    pub pick_mode: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RampProfile {
    Linear,
    Eased,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RampShape {
    Straight,
    Curved,
    Helix,
}

#[derive(Debug, Clone)]
pub struct RampBreakdownRow {
    pub piece_id: String,
    pub catalog_id: String,
    pub length_mm: f64,
    pub start_h_mm: f64,
    pub end_h_mm: f64,
    pub grade_pct: f64,
    pub effective_grade_pct: f64,
    pub is_s_curve: bool,
}

#[derive(Debug, Clone)]
pub struct RampSummary {
    pub piece_count: usize,
    pub total_length_mm: f64,
    pub total_rise_mm: f64,
    pub avg_gradient_pct: f64,
    pub max_gradient_pct: f64,
    pub effective_max_pct: f64,
    pub has_s_curve: bool,
    pub is_continuous: bool,
}

#[derive(Debug, Clone)]
pub struct StartConnectorOption {
    pub piece_id: String,
    pub catalog_id: String,
    pub connector_index: usize,
    pub world_x: f64,
    pub world_y: f64,
    pub world_z: f64,
    pub angle_deg: f64,
    pub is_selected: bool,
}

pub fn min_clearance_for_scale(scale: &str) -> f64 {
    match scale.trim().to_uppercase().as_str() {
        "Z" => 40.0,
        "N" => 50.0,
        "TT" | "H0M" => 60.0,
        "H0" => 80.0,
        "0" => 120.0,
        "1" => 170.0,
        "G" => 220.0,
        _ => 80.0,
    }
}

pub fn piece_length_mm(piece: &TrackPiece, catalogs: &[Catalog]) -> f64 {
    if let Some(cl) = piece.custom_length {
        if cl > 1.0 {
            return cl;
        }
    }
    if let Some(e) = find_entry(catalogs, &piece.catalog_id) {
        if let Some(len) = e.length {
            if len > 1.0 {
                return len;
            }
        }
        if let (Some(r), Some(a)) = (e.radius, e.angle) {
            let arc = r * deg2rad(a.abs());
            if arc > 5.0 {
                return arc;
            }
        }
    }
    if piece.connectors.len() >= 2 {
        let d = (piece.connectors[0].x - piece.connectors[1].x)
            .hypot(piece.connectors[0].y - piece.connectors[1].y);
        if d > 5.0 {
            return d;
        }
    }
    188.3
}

pub(super) fn entry_length_mm(e: &CatalogEntry) -> f64 {
    if let Some(len) = e.length {
        if len > 1.0 {
            return len;
        }
    }
    if let (Some(r), Some(a)) = (e.radius, e.angle) {
        let arc = r * deg2rad(a.abs());
        if arc > 5.0 {
            return arc;
        }
    }
    188.3
}

pub(super) fn layer_height_of(layout: &Layout, layer_id: Option<&str>) -> f64 {
    let lid = layer_id.unwrap_or("layer-1");
    layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == lid))
        .and_then(|l| l.height_mm)
        .unwrap_or(0.0)
}

/// Orders selected track pieces into a contiguous chain and determines whether each piece
/// is traversed in reverse (entering at connector 1 instead of connector 0).
pub fn order_selected_chain(layout: &Layout, selected: &HashSet<String>) -> Vec<(String, bool)> {
    let sel_pieces: Vec<&TrackPiece> = layout
        .pieces
        .iter()
        .filter(|p| selected.contains(&p.id))
        .collect();
    if sel_pieces.is_empty() {
        return Vec::new();
    }
    if sel_pieces.len() == 1 {
        return vec![(sel_pieces[0].id.clone(), false)];
    }

    let mut id_map: HashMap<&str, usize> = HashMap::new();
    for (i, p) in sel_pieces.iter().enumerate() {
        id_map.insert(p.id.as_str(), i);
    }

    // (neighbor_idx, my_conn_idx, nbr_conn_idx)
    let mut adj: Vec<Vec<(usize, usize, usize)>> = vec![Vec::new(); sel_pieces.len()];
    for (i, p1) in sel_pieces.iter().enumerate() {
        for (j, p2) in sel_pieces.iter().enumerate().skip(i + 1) {
            for (c1_idx, c1) in p1.connectors.iter().enumerate() {
                for (c2_idx, c2) in p2.connectors.iter().enumerate() {
                    let explicit = c1
                        .connected_to
                        .as_ref()
                        .map_or(false, |r| r.piece_id == p2.id && r.connector_index == c2_idx);
                    if explicit || (c1.x - c2.x).hypot(c1.y - c2.y) <= 8.0 {
                        adj[i].push((j, c1_idx, c2_idx));
                        adj[j].push((i, c2_idx, c1_idx));
                    }
                }
            }
        }
    }

    // Pick leftmost endpoint (degree <= 1) if one exists
    let mut endpoints: Vec<usize> = (0..sel_pieces.len()).filter(|&i| adj[i].len() <= 1).collect();
    endpoints.sort_by(|&a, &b| {
        sel_pieces[a]
            .transform
            .x
            .partial_cmp(&sel_pieces[b].transform.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                sel_pieces[a]
                    .transform
                    .y
                    .partial_cmp(&sel_pieces[b].transform.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let start_idx = endpoints.first().copied().unwrap_or(0);

    let mut visited = vec![false; sel_pieces.len()];
    let mut ordered = Vec::with_capacity(sel_pieces.len());
    let mut cur = Some(start_idx);
    let mut entry_conn = if let Some(&(_, out_c, _)) = adj[start_idx].first() {
        if out_c == 0 { 1 } else { 0 }
    } else {
        0
    };

    while let Some(idx) = cur {
        if visited[idx] {
            break;
        }
        visited[idx] = true;
        ordered.push((sel_pieces[idx].id.clone(), entry_conn == 1));

        let next = adj[idx].iter().find(|&&(nbr, _, _)| !visited[nbr]).copied();
        if let Some((nbr_idx, _, nbr_entry_conn)) = next {
            cur = Some(nbr_idx);
            entry_conn = nbr_entry_conn;
        } else {
            cur = None;
        }
    }

    for (i, p) in sel_pieces.iter().enumerate() {
        if !visited[i] {
            ordered.push((p.id.clone(), false));
        }
    }

    ordered
}

/// Finds the shortest connected track chain between `start_id` and `end_id` across the entire layout,
/// returning `(piece_id, reversed)` where `reversed == true` means traversal enters at connector 1
/// and exits at connector 0.
pub fn find_ramp_chain_between(
    layout: &Layout,
    start_id: &str,
    end_id: &str,
) -> Vec<(String, bool)> {
    let n = layout.pieces.len();
    let Some(start_idx) = layout.pieces.iter().position(|p| p.id == start_id) else {
        return Vec::new();
    };
    let Some(end_idx) = layout.pieces.iter().position(|p| p.id == end_id) else {
        return Vec::new();
    };
    if start_idx == end_idx {
        return vec![(start_id.to_string(), false)];
    }

    let id_to_idx: HashMap<&str, usize> = layout
        .pieces
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();

    // Adjacency: (neighbor_idx, my_conn_idx, nbr_conn_idx)
    let mut adj: Vec<Vec<(usize, usize, usize)>> = vec![Vec::new(); n];
    for (i, p) in layout.pieces.iter().enumerate() {
        for (c_idx, c) in p.connectors.iter().enumerate() {
            if let Some(ref link) = c.connected_to {
                if let Some(&j) = id_to_idx.get(link.piece_id.as_str()) {
                    if j != i && !adj[i].iter().any(|&(nbr, _, _)| nbr == j) {
                        adj[i].push((j, c_idx, link.connector_index));
                    }
                }
            }
        }
    }
    // Proximity fallback (<= 8 mm) in case two adjacent tracks weren't explicitly linked yet
    for i in 0..n {
        for j in (i + 1)..n {
            if adj[i].iter().any(|&(nbr, _, _)| nbr == j) {
                continue;
            }
            'pair: for (c1_idx, c1) in layout.pieces[i].connectors.iter().enumerate() {
                for (c2_idx, c2) in layout.pieces[j].connectors.iter().enumerate() {
                    if (c1.x - c2.x).hypot(c1.y - c2.y) <= 8.0 {
                        adj[i].push((j, c1_idx, c2_idx));
                        adj[j].push((i, c2_idx, c1_idx));
                        break 'pair;
                    }
                }
            }
        }
    }

    // BFS shortest path from start_idx to end_idx
    let mut prev: Vec<Option<(usize, usize, usize)>> = vec![None; n];
    let mut visited = vec![false; n];
    let mut queue = VecDeque::new();
    visited[start_idx] = true;
    queue.push_back(start_idx);

    while let Some(u) = queue.pop_front() {
        if u == end_idx {
            break;
        }
        for &(v, u_conn, v_conn) in &adj[u] {
            if !visited[v] {
                visited[v] = true;
                prev[v] = Some((u, u_conn, v_conn));
                queue.push_back(v);
            }
        }
    }

    if !visited[end_idx] {
        return Vec::new();
    }

    // Reconstruct path of (piece_idx, out_conn_to_next, in_conn_from_prev)
    let mut path_indices = Vec::new();
    let mut cur = end_idx;
    while cur != start_idx {
        path_indices.push(cur);
        cur = prev[cur].unwrap().0;
    }
    path_indices.push(start_idx);
    path_indices.reverse();

    let mut result = Vec::with_capacity(path_indices.len());
    for pos in 0..path_indices.len() {
        let idx = path_indices[pos];
        let reversed = if pos + 1 < path_indices.len() {
            let next_idx = path_indices[pos + 1];
            let out_conn = prev[next_idx].map(|(_, u_c, _)| u_c).unwrap_or(1);
            // Exiting through connector 0 means we entered at connector 1 (reversed)
            out_conn == 0
        } else {
            let in_conn = prev[idx].map(|(_, _, v_c)| v_c).unwrap_or(0);
            in_conn == 1
        };
        result.push((layout.pieces[idx].id.clone(), reversed));
    }
    result
}

/// Resolves the ordered ramp chain from explicit Start/End pieces if set, otherwise from `selected`.
pub fn resolve_ramp_chain(
    layout: &Layout,
    selected: &HashSet<String>,
    start_id: Option<&str>,
    end_id: Option<&str>,
) -> Vec<(String, bool)> {
    if let (Some(s), Some(e)) = (start_id, end_id) {
        let path = find_ramp_chain_between(layout, s, e);
        if !path.is_empty() {
            return path;
        }
    }
    let mut chain = order_selected_chain(layout, selected);
    if chain.len() >= 2 {
        let should_reverse = match (start_id, end_id) {
            (Some(s), _) if chain.last().is_some_and(|(id, _)| id == s) => true,
            (_, Some(e)) if chain.first().is_some_and(|(id, _)| id == e) => true,
            _ => false,
        };
        if should_reverse {
            chain.reverse();
            for (_, rev) in &mut chain {
                *rev = !*rev;
            }
        }
    }
    chain
}

/// Returns `(entry_world, exit_world)` points for each piece along a resolved ramp chain
/// so the 2D canvas can render clear directional arrows from Start (A) to End (B).
pub fn ramp_chain_direction_segments(
    layout: &Layout,
    chain: &[(String, bool)],
) -> Vec<(Vec2, Vec2)> {
    let mut out = Vec::with_capacity(chain.len());
    for (pid, rev) in chain {
        if let Some(p) = layout.pieces.iter().find(|pc| &pc.id == pid) {
            if p.connectors.len() >= 2 {
                let (c_in, c_out) = if *rev {
                    (&p.connectors[1], &p.connectors[0])
                } else {
                    (&p.connectors[0], &p.connectors[1])
                };
                out.push((
                    Vec2 { x: c_in.x, y: c_in.y },
                    Vec2 { x: c_out.x, y: c_out.y },
                ));
            }
        }
    }
    out
}

/// Checks whether every consecutive pair of pieces in `chain` is directly connected
/// (either via explicit `connected_to` link or connector proximity <= 8.0 mm).
pub fn is_chain_continuous(layout: &Layout, chain: &[(String, bool)]) -> bool {
    if chain.len() <= 1 {
        return true;
    }
    for pair in chain.windows(2) {
        let Some(p1) = layout.pieces.iter().find(|p| p.id == pair[0].0) else {
            return false;
        };
        let Some(p2) = layout.pieces.iter().find(|p| p.id == pair[1].0) else {
            return false;
        };
        let mut connected = false;
        'outer: for (c1_idx, c1) in p1.connectors.iter().enumerate() {
            for (c2_idx, c2) in p2.connectors.iter().enumerate() {
                let explicit = c1
                    .connected_to
                    .as_ref()
                    .is_some_and(|r| r.piece_id == p2.id && r.connector_index == c2_idx)
                    || c2
                        .connected_to
                        .as_ref()
                        .is_some_and(|r| r.piece_id == p1.id && r.connector_index == c1_idx);
                if explicit || (c1.x - c2.x).hypot(c1.y - c2.y) <= 8.0 {
                    connected = true;
                    break 'outer;
                }
            }
        }
        if !connected {
            return false;
        }
    }
    true
}

/// Returns a piece's world height (layer height + local start/end elevation) rounded to 0.1 mm.
pub fn piece_world_height_mm(layout: &Layout, piece_id: &str, at_end: bool) -> Option<f64> {
    let p = layout.pieces.iter().find(|pc| pc.id == piece_id)?;
    let layer_h = layer_height_of(layout, p.layer_id.as_deref());
    let local_h = match p.elevation {
        Some(el) => {
            if at_end {
                el.end
            } else {
                el.start
            }
        }
        None => 0.0,
    };
    Some(((layer_h + local_h) * 10.0).round() / 10.0)
}

/// Clamps `end_h` so the average slope over `total_length_mm` never exceeds `max_grade_pct`.
pub fn compute_auto_fit_end_height(
    start_h: f64,
    end_h: f64,
    total_length_mm: f64,
    max_grade_pct: f64,
) -> f64 {
    if total_length_mm <= 1.0 {
        return end_h;
    }
    let max_rise = total_length_mm * (max_grade_pct.clamp(0.5, 12.0) / 100.0);
    if (end_h - start_h).abs() > max_rise {
        ((start_h + (end_h - start_h).signum() * max_rise) * 2.0).round() / 2.0
    } else {
        end_h
    }
}


mod auto_ramp;
mod generator;
#[cfg(test)]
mod tests;

pub use auto_ramp::*;
pub use generator::*;
