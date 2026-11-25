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

fn entry_length_mm(e: &CatalogEntry) -> f64 {
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

fn layer_height_of(layout: &Layout, layer_id: Option<&str>) -> f64 {
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

pub fn preview_auto_ramp_for_chain(
    layout: &Layout,
    catalogs: &[Catalog],
    chain: &[(String, bool)],
    start_h: f64,
    end_h: f64,
    profile: RampProfile,
) -> Option<RampSummary> {
    if chain.is_empty() {
        return None;
    }
    let lengths: Vec<f64> = chain
        .iter()
        .filter_map(|(id, _)| layout.pieces.iter().find(|p| &p.id == id))
        .map(|p| piece_length_mm(p, catalogs))
        .collect();
    if lengths.is_empty() {
        return None;
    }
    let total_len: f64 = lengths.iter().sum();
    let total_rise = end_h - start_h;
    let avg_grade = if total_len > 0.0 {
        (total_rise.abs() / total_len) * 100.0
    } else {
        0.0
    };

    let max_grade = match profile {
        RampProfile::Linear => avg_grade,
        RampProfile::Eased => {
            let mut max_g = 0.0_f64;
            let mut accum = 0.0_f64;
            for &len in &lengths {
                let u0 = if total_len > 0.0 { accum / total_len } else { 0.0 };
                let u1 = if total_len > 0.0 { (accum + len) / total_len } else { 1.0 };
                accum += len;
                let s0 = u0 * u0 * (3.0 - 2.0 * u0);
                let s1 = u1 * u1 * (3.0 - 2.0 * u1);
                let g = if len > 0.0 {
                    ((total_rise * (s1 - s0)).abs() / len) * 100.0
                } else {
                    0.0
                };
                if g > max_g {
                    max_g = g;
                }
            }
            max_g
        }
    };

    let mut max_eff = max_grade;
    let mut has_s_curve = false;
    for row in build_ramp_breakdown(layout, catalogs, chain, start_h, end_h, profile) {
        if row.is_s_curve {
            has_s_curve = true;
        }
        if row.effective_grade_pct > max_eff {
            max_eff = row.effective_grade_pct;
        }
    }

    Some(RampSummary {
        piece_count: lengths.len(),
        total_length_mm: (total_len * 10.0).round() / 10.0,
        total_rise_mm: (total_rise * 10.0).round() / 10.0,
        avg_gradient_pct: (avg_grade * 10.0).round() / 10.0,
        max_gradient_pct: (max_grade * 10.0).round() / 10.0,
        effective_max_pct: (max_eff * 10.0).round() / 10.0,
        has_s_curve,
    })
}

fn piece_curve_sign(piece: &TrackPiece, rev: bool, catalogs: &[Catalog]) -> i8 {
    let entry = find_entry(catalogs, &piece.catalog_id);
    if let Some(ca) = piece.custom_angle {
        if ca.abs() > 1.0 {
            let s = if ca > 0.0 { 1 } else { -1 };
            return if rev { -s } else { s };
        }
    }
    if let Some(e) = entry {
        if e.kind == "curve" {
            let a = e.angle.unwrap_or(30.0);
            let s = if a >= 0.0 { 1 } else { -1 };
            return if rev { -s } else { s };
        }
    }
    0
}

pub fn build_ramp_breakdown(
    layout: &Layout,
    catalogs: &[Catalog],
    chain: &[(String, bool)],
    start_h: f64,
    end_h: f64,
    profile: RampProfile,
) -> Vec<RampBreakdownRow> {
    if chain.is_empty() {
        return Vec::new();
    }
    let items: Vec<(&TrackPiece, bool, f64)> = chain
        .iter()
        .filter_map(|(id, rev)| {
            layout
                .pieces
                .iter()
                .find(|p| &p.id == id)
                .map(|p| (p, *rev, piece_length_mm(p, catalogs)))
        })
        .collect();
    if items.is_empty() {
        return Vec::new();
    }
    let total_len: f64 = items.iter().map(|(_, _, l)| *l).sum();
    let total_rise = end_h - start_h;
    let signs: Vec<i8> = items
        .iter()
        .map(|(p, rev, _)| piece_curve_sign(p, *rev, catalogs))
        .collect();

    let mut rows = Vec::with_capacity(items.len());
    let mut accum = 0.0_f64;
    for (i, (p, _, len)) in items.iter().enumerate() {
        let u0 = if total_len > 0.0 { accum / total_len } else { 0.0 };
        let u1 = if total_len > 0.0 { (accum + len) / total_len } else { 1.0 };
        accum += len;
        let (f0, f1) = match profile {
            RampProfile::Linear => (u0, u1),
            RampProfile::Eased => (u0 * u0 * (3.0 - 2.0 * u0), u1 * u1 * (3.0 - 2.0 * u1)),
        };
        let sh = start_h + total_rise * f0;
        let eh = start_h + total_rise * f1;
        let g = if *len > 0.0 {
            ((eh - sh).abs() / *len) * 100.0
        } else {
            0.0
        };
        let is_s = (i > 0 && signs[i] != 0 && signs[i - 1] != 0 && signs[i] == -signs[i - 1])
            || (i + 1 < signs.len() && signs[i] != 0 && signs[i + 1] != 0 && signs[i] == -signs[i + 1]);
        let eff_g = if is_s { g * 1.5 } else { g };
        rows.push(RampBreakdownRow {
            piece_id: p.id.clone(),
            catalog_id: p.catalog_id.clone(),
            length_mm: (*len * 10.0).round() / 10.0,
            start_h_mm: (sh * 10.0).round() / 10.0,
            end_h_mm: (eh * 10.0).round() / 10.0,
            grade_pct: (g * 10.0).round() / 10.0,
            effective_grade_pct: (eff_g * 10.0).round() / 10.0,
            is_s_curve: is_s,
        });
    }
    rows
}

pub fn calculate_end_height_from_max_gradient(
    layout: &Layout,
    catalogs: &[Catalog],
    chain: &[(String, bool)],
    start_h: f64,
    max_grade_pct: f64,
    slope_up: bool,
    profile: RampProfile,
) -> f64 {
    let total_len: f64 = chain
        .iter()
        .filter_map(|(id, _)| layout.pieces.iter().find(|p| &p.id == id))
        .map(|p| piece_length_mm(p, catalogs))
        .sum();
    if total_len <= 1.0 {
        return start_h;
    }
    let dir = if slope_up { 1.0 } else { -1.0 };
    // Eased peak slope is 1.5x average slope, so limit rise accordingly
    let profile_factor = if profile == RampProfile::Eased { 1.5 } else { 1.0 };
    let max_rise = (total_len * (max_grade_pct.max(0.1) / 100.0)) / profile_factor;
    ((start_h + dir * max_rise) * 10.0).round() / 10.0
}

/// Infers `(chain, start_h_mm, end_h_mm)` from unselected adjacent tracks connected at the
/// start and end of the selected chain.
pub fn calculate_auto_ramp(
    layout: &Layout,
    selected: &HashSet<String>,
    catalogs: &[Catalog],
) -> Option<(Vec<(String, bool)>, f64, f64)> {
    let _ = catalogs;
    let chain = order_selected_chain(layout, selected);
    if chain.is_empty() {
        return None;
    }
    let chain_ids: HashSet<&str> = chain.iter().map(|(id, _)| id.as_str()).collect();
    let neighbor_height = |pid: &str, conn_idx: usize| -> Option<f64> {
        let p = layout.pieces.iter().find(|pc| pc.id == pid)?;
        let c = p.connectors.get(conn_idx)?;
        if let Some(ref link) = c.connected_to {
            if !chain_ids.contains(link.piece_id.as_str()) {
                if let Some(nbr) = layout.pieces.iter().find(|pc| pc.id == link.piece_id) {
                    let lh = layer_height_of(layout, nbr.layer_id.as_deref());
                    return Some(railbench_core::snapping::piece_connector_height(
                        nbr,
                        link.connector_index,
                        lh,
                    ));
                }
            }
        }
        None
    };

    let (first_id, first_rev) = &chain[0];
    let (last_id, last_rev) = &chain[chain.len() - 1];
    let first_in_conn = if *first_rev { 1 } else { 0 };
    let last_out_conn = if *last_rev { 0 } else { 1 };

    let start_h = neighbor_height(first_id, first_in_conn).unwrap_or_else(|| {
        let p = layout.pieces.iter().find(|pc| &pc.id == first_id).unwrap();
        let lh = layer_height_of(layout, p.layer_id.as_deref());
        railbench_core::snapping::piece_connector_height(p, first_in_conn, lh)
    });
    let end_h = neighbor_height(last_id, last_out_conn).unwrap_or_else(|| {
        let p = layout.pieces.iter().find(|pc| &pc.id == last_id).unwrap();
        let lh = layer_height_of(layout, p.layer_id.as_deref());
        railbench_core::snapping::piece_connector_height(p, last_out_conn, lh)
    });

    Some((chain, start_h, end_h))
}

#[allow(dead_code)]
pub fn preview_auto_ramp(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    start_h: f64,
    end_h: f64,
    profile: RampProfile,
) -> Option<RampSummary> {
    let chain = order_selected_chain(layout, selected);
    preview_auto_ramp_for_chain(layout, catalogs, &chain, start_h, end_h, profile)
}

pub fn apply_auto_ramp_for_chain(
    layout: &mut Layout,
    catalogs: &[Catalog],
    chain: &[(String, bool)],
    start_h: f64,
    end_h: f64,
    profile: RampProfile,
) -> Result<RampSummary, String> {
    if chain.is_empty() {
        return Err("Select a start & end track or a track chain first.".into());
    }

    let lengths: Vec<f64> = chain
        .iter()
        .filter_map(|(id, _)| layout.pieces.iter().find(|p| &p.id == id))
        .map(|p| piece_length_mm(p, catalogs))
        .collect();
    let rev_flags: Vec<u8> = chain.iter().map(|(_, rev)| if *rev { 1 } else { 0 }).collect();
    let total_len: f64 = lengths.iter().sum();
    let total_rise = end_h - start_h;

    let world_elevs = match profile {
        RampProfile::Linear => compute_ramp_chain_elevations(&lengths, &rev_flags, start_h, total_rise, 0.0),
        RampProfile::Eased => {
            let mut out = vec![0.0_f64; lengths.len() * 2];
            let mut accum = 0.0_f64;
            for (i, &len) in lengths.iter().enumerate() {
                let u0 = if total_len > 0.0 { accum / total_len } else { 0.0 };
                let u1 = if total_len > 0.0 { (accum + len) / total_len } else { 1.0 };
                accum += len;
                let s0 = u0 * u0 * (3.0 - 2.0 * u0);
                let s1 = u1 * u1 * (3.0 - 2.0 * u1);
                let h_entry = start_h + total_rise * s0;
                let h_exit = start_h + total_rise * s1;
                let (hc0, hc1) = if rev_flags[i] != 0 { (h_exit, h_entry) } else { (h_entry, h_exit) };
                out[i * 2] = hc0;
                out[i * 2 + 1] = hc1;
            }
            out
        }
    };

    for (i, (pid, _)) in chain.iter().enumerate() {
        let base_h = layout
            .pieces
            .iter()
            .find(|p| &p.id == pid)
            .map(|p| layer_height_of(layout, p.layer_id.as_deref()))
            .unwrap_or(0.0);
        if let Some(p) = layout.pieces.iter_mut().find(|p| &p.id == pid) {
            let s = ((world_elevs[i * 2] - base_h) * 100.0).round() / 100.0;
            let e = ((world_elevs[i * 2 + 1] - base_h) * 100.0).round() / 100.0;
            p.elevation = Some(Elevation { start: s, end: e });
        }
    }

    update_layout_connections(layout, 4.0, 35.0, 1.0);
    disconnect_cross_type_connections(layout, catalogs);

    preview_auto_ramp_for_chain(layout, catalogs, chain, start_h, end_h, profile)
        .ok_or_else(|| "Unable to compute ramp.".into())
}

#[allow(dead_code)]
pub fn apply_auto_ramp(
    layout: &mut Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    start_h: f64,
    end_h: f64,
    profile: RampProfile,
) -> Result<RampSummary, String> {
    let chain = order_selected_chain(layout, selected);
    apply_auto_ramp_for_chain(layout, catalogs, &chain, start_h, end_h, profile)
}

pub fn available_curves_for_catalog(catalog: &Catalog) -> Vec<CatalogEntry> {
    let mut curves: Vec<CatalogEntry> = catalog
        .entries
        .iter()
        .filter(|e| e.kind == "curve" && e.radius.unwrap_or(0.0) > 40.0)
        .cloned()
        .collect();
    curves.sort_by(|a, b| {
        a.radius
            .unwrap_or(0.0)
            .partial_cmp(&b.radius.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                (a.angle.unwrap_or(30.0) - 30.0)
                    .abs()
                    .partial_cmp(&(b.angle.unwrap_or(30.0) - 30.0).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    curves
}

pub fn available_curve_radii_dedup(catalog: &Catalog) -> Vec<CatalogEntry> {
    let all = available_curves_for_catalog(catalog);
    let mut seen_r: Vec<i32> = Vec::new();
    let mut out = Vec::new();
    for c in all {
        let r_key = c.radius.unwrap_or(0.0).round() as i32;
        if !seen_r.contains(&r_key) {
            seen_r.push(r_key);
            out.push(c);
        }
    }
    out
}

pub fn available_open_connectors_for_piece(layout: &Layout, piece_id: &str) -> Vec<usize> {
    let Some(anchor) = layout.pieces.iter().find(|p| p.id == piece_id) else {
        return Vec::new();
    };
    anchor
        .connectors
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.connected_to.is_none()
                && !layout.pieces.iter().any(|other| {
                    other.id != anchor.id
                        && other
                            .connectors
                            .iter()
                            .any(|oc| (c.x - oc.x).hypot(c.y - oc.y) <= 6.0)
                })
        })
        .map(|(ci, _)| ci)
        .collect()
}

#[cfg_attr(not(test), allow(dead_code))]
#[allow(clippy::too_many_arguments)]
pub fn generate_ramp_or_helix(
    layout: &mut Layout,
    catalogs: &[Catalog],
    active_catalog_idx: usize,
    next_seq: &mut u64,
    anchor_id: &str,
    shape: RampShape,
    curve_left: bool,
    target_height_mm: f64,
    max_grade_pct: f64,
    turns: f64,
    selected_curve_id: Option<&str>,
) -> Result<(Vec<String>, RampSummary), String> {
    generate_ramp_or_helix_ext(
        layout,
        catalogs,
        active_catalog_idx,
        next_seq,
        anchor_id,
        None,
        None,
        shape,
        curve_left,
        180.0,
        target_height_mm,
        max_grade_pct,
        turns,
        selected_curve_id,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn generate_ramp_or_helix_ext(
    layout: &mut Layout,
    catalogs: &[Catalog],
    active_catalog_idx: usize,
    next_seq: &mut u64,
    anchor_id: &str,
    preferred_conn_idx: Option<usize>,
    target_layer_id: Option<&str>,
    shape: RampShape,
    curve_left: bool,
    arc_angle_deg: f64,
    target_height_mm: f64,
    max_grade_pct: f64,
    turns: f64,
    selected_curve_id: Option<&str>,
) -> Result<(Vec<String>, RampSummary), String> {
    let anchor = layout
        .pieces
        .iter()
        .find(|p| p.id == anchor_id)
        .cloned()
        .ok_or_else(|| "Select one anchor track first.".to_string())?;

    let open_list = available_open_connectors_for_piece(layout, anchor_id);
    let chosen_ci = preferred_conn_idx
        .filter(|ci| open_list.contains(ci))
        .or_else(|| open_list.last().copied())
        .ok_or_else(|| "Selected anchor track has no open connector.".to_string())?;
    let start_conn = &anchor.connectors[chosen_ci];

    let anchor_layer_h = layer_height_of(layout, anchor.layer_id.as_deref());
    let anchor_local_h = match anchor.elevation {
        Some(el) => if chosen_ci == 0 { el.start } else { el.end },
        None => 0.0,
    };
    let h_start = anchor_layer_h + anchor_local_h;
    let rise = target_height_mm - h_start;
    if rise.abs() < 0.2 {
        return Err("Target height matches the anchor connector height already.".into());
    }

    let place_layer_id = target_layer_id
        .map(|s| s.to_string())
        .or_else(|| anchor.layer_id.clone());
    let place_layer_h = layer_height_of(layout, place_layer_id.as_deref());

    // Pick source catalog (matching anchor piece or active catalog)
    let cat = catalogs
        .iter()
        .find(|c| c.entries.iter().any(|e| e.id == anchor.catalog_id))
        .or_else(|| catalogs.get(active_catalog_idx))
        .ok_or_else(|| "No catalog available.".to_string())?;

    let mut straights: Vec<&CatalogEntry> = cat
        .entries
        .iter()
        .filter(|e| e.kind == "straight" && e.length.unwrap_or(0.0) > 40.0)
        .collect();
    straights.sort_by(|a, b| {
        (a.length.unwrap_or(0.0) - 188.3)
            .abs()
            .partial_cmp(&(b.length.unwrap_or(0.0) - 188.3).abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let curves = available_curves_for_catalog(cat);

    let straight_entry = straights.first().copied();
    let curve_entry = selected_curve_id
        .and_then(|cid| cat.entries.iter().find(|e| e.id == cid))
        .or_else(|| curves.iter().min_by(|a, b| {
            (a.angle.unwrap_or(30.0) - 30.0)
                .abs()
                .partial_cmp(&(b.angle.unwrap_or(30.0) - 30.0).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        }));

    let grade = max_grade_pct.clamp(0.5, 12.0);
    let min_req_len = rise.abs() / (grade / 100.0);

    let mut sequence: Vec<(&CatalogEntry, bool)> = Vec::new();
    match shape {
        RampShape::Straight => {
            let e = straight_entry.or(curve_entry).ok_or_else(|| "No straight piece in catalog.".to_string())?;
            let count = ((min_req_len / entry_length_mm(e)).ceil() as usize).clamp(1, 48);
            for _ in 0..count {
                sequence.push((e, false));
            }
        }
        RampShape::Curved => {
            let c_e = curve_entry.or(straight_entry).ok_or_else(|| "No curve piece in catalog.".to_string())?;
            let c_len = entry_length_mm(c_e);
            let c_ang = c_e.angle.unwrap_or(30.0).max(5.0);
            let target_arc = arc_angle_deg.clamp(45.0, 270.0);
            let arc_curves = (target_arc / c_ang).round().max(2.0) as usize;
            let curve_total_len = arc_curves as f64 * c_len;
            if min_req_len > curve_total_len + 10.0 {
                if let Some(s_e) = straight_entry {
                    let deficit = min_req_len - curve_total_len;
                    let total_straights = (deficit / entry_length_mm(s_e)).ceil() as usize;
                    let lead_in = total_straights / 2;
                    let lead_out = total_straights - lead_in;
                    for _ in 0..lead_in {
                        sequence.push((s_e, false));
                    }
                    for _ in 0..arc_curves {
                        sequence.push((c_e, curve_left));
                    }
                    for _ in 0..lead_out {
                        sequence.push((s_e, false));
                    }
                } else {
                    let count = ((min_req_len / c_len).ceil() as usize).max(arc_curves).clamp(2, 36);
                    for _ in 0..count {
                        sequence.push((c_e, curve_left));
                    }
                }
            } else {
                for _ in 0..arc_curves {
                    sequence.push((c_e, curve_left));
                }
            }
        }
        RampShape::Helix => {
            let c_e = curve_entry.ok_or_else(|| "No curve piece in catalog for helix.".to_string())?;
            let c_len = entry_length_mm(c_e);
            let c_ang = c_e.angle.unwrap_or(30.0).max(5.0);
            let by_turns = ((turns.max(0.5) * 360.0) / c_ang).ceil() as usize;
            let by_grade = (min_req_len / c_len).ceil() as usize;
            let count = by_turns.max(by_grade).clamp(6, 72);
            for _ in 0..count {
                sequence.push((c_e, curve_left));
            }
        }
    }

    let lengths: Vec<f64> = sequence.iter().map(|(e, _)| entry_length_mm(e)).collect();
    let left_flags: Vec<u8> = sequence.iter().map(|(_, left)| if *left { 1 } else { 0 }).collect();
    let elevs = compute_ramp_chain_elevations(&lengths, &left_flags, h_start, rise, place_layer_h);
    let total_len: f64 = lengths.iter().sum();

    let mut curr_x = start_conn.x;
    let mut curr_y = start_conn.y;
    let mut curr_ang = start_conn.angle;
    let mut new_ids = Vec::with_capacity(sequence.len());

    for (i, (entry, is_left)) in sequence.iter().enumerate() {
        let in_idx = if *is_left { 1 } else { 0 };
        let out_idx = if *is_left { 0 } else { 1 };
        let locals = local_connectors(entry);
        let in_loc = &locals[in_idx.min(locals.len() - 1)];

        let rot = norm_angle_deg(curr_ang + 180.0 - in_loc.angle);
        let ro = rotate(Vec2 { x: in_loc.x, y: in_loc.y }, rot);
        let tr = Transform {
            x: curr_x - ro.x,
            y: curr_y - ro.y,
            rotation: rot,
        };

        let id = format!("ramp-{}", *next_seq);
        *next_seq += 1;
        let mut piece = create_piece(id.clone(), entry, tr, place_layer_id.clone());
        piece.elevation = Some(Elevation {
            start: elevs[i * 2],
            end: elevs[i * 2 + 1],
        });

        let out_c = &piece.connectors[out_idx.min(piece.connectors.len() - 1)];
        curr_x = out_c.x;
        curr_y = out_c.y;
        curr_ang = out_c.angle;

        new_ids.push(id);
        layout.pieces.push(piece);
    }

    update_layout_connections(layout, 4.0, 35.0, 2.0);
    disconnect_cross_type_connections(layout, catalogs);

    let avg_grade = if total_len > 0.0 {
        (rise.abs() / total_len) * 100.0
    } else {
        0.0
    };
    let summary = RampSummary {
        piece_count: sequence.len(),
        total_length_mm: (total_len * 10.0).round() / 10.0,
        total_rise_mm: (rise * 10.0).round() / 10.0,
        avg_gradient_pct: (avg_grade * 10.0).round() / 10.0,
        max_gradient_pct: (avg_grade * 10.0).round() / 10.0,
        effective_max_pct: (avg_grade * 10.0).round() / 10.0,
        has_s_curve: false,
    };
    Ok((new_ids, summary))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::builtin_catalogs;

    fn make_layout_with_chain(n: usize) -> (Layout, Vec<Catalog>, HashSet<String>) {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "24188").unwrap();
        let mut pieces = Vec::new();
        let mut sel = HashSet::new();
        for i in 0..n {
            let id = format!("s{i}");
            let mut p = create_piece(
                id.clone(),
                entry,
                Transform { x: i as f64 * 188.3, y: 0.0, rotation: 0.0 },
                Some("layer-1".into()),
            );
            p.elevation = None;
            sel.insert(id);
            pieces.push(p);
        }
        let mut layout = Layout {
            name: "RampTest".into(),
            scale: "H0".into(),
            pieces,
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
            active_layer_id: Some("layer-1".into()),
            baseboard: None,
            baseboards: None,
            terrain_field: None,
        };
        update_layout_connections(&mut layout, 4.0, 35.0, 2.0);
        (layout, catalogs, sel)
    }

    #[test]
    fn auto_ramp_linear_chain() {
        let (mut layout, catalogs, sel) = make_layout_with_chain(4);
        let sum = apply_auto_ramp(&mut layout, &catalogs, &sel, 0.0, 80.0, RampProfile::Linear).unwrap();
        assert_eq!(sum.piece_count, 4);
        for i in 0..3 {
            let e_cur = layout.pieces[i].elevation.unwrap().end;
            let s_nxt = layout.pieces[i + 1].elevation.unwrap().start;
            assert!((e_cur - s_nxt).abs() < 0.02);
        }
        assert!((layout.pieces[3].elevation.unwrap().end - 80.0).abs() < 0.02);
    }

    #[test]
    fn generate_straight_ramp_and_helix() {
        let (mut layout, catalogs, _) = make_layout_with_chain(1);
        let mut seq = 1u64;
        let (ids, sum) = generate_ramp_or_helix(
            &mut layout,
            &catalogs,
            0,
            &mut seq,
            "s0",
            RampShape::Straight,
            false,
            80.0,
            3.0,
            1.0,
            None,
        )
        .unwrap();
        assert!(!ids.is_empty());
        assert!((sum.total_rise_mm - 80.0).abs() < 0.1);
        let last = layout.pieces.last().unwrap().elevation.unwrap();
        assert!((last.end - 80.0).abs() < 0.1);

        let (h_ids, h_sum) = generate_ramp_or_helix(
            &mut layout,
            &catalogs,
            0,
            &mut seq,
            ids.last().unwrap(),
            RampShape::Helix,
            true,
            160.0,
            3.0,
            1.0,
            None,
        )
        .unwrap();
        assert!(h_ids.len() >= 6);
        assert!((h_sum.total_rise_mm - 80.0).abs() < 0.1);
    }

    #[test]
    fn ramp_start_end_path_and_reverse_direction() {
        let (mut layout, catalogs, _) = make_layout_with_chain(4);
        let empty_sel = HashSet::new();

        // Forward path s0 -> s3 (without needing s1, s2 selected!)
        let fwd = resolve_ramp_chain(&layout, &empty_sel, Some("s0"), Some("s3"));
        assert_eq!(
            fwd,
            vec![
                ("s0".into(), false),
                ("s1".into(), false),
                ("s2".into(), false),
                ("s3".into(), false),
            ]
        );
        let segs_fwd = ramp_chain_direction_segments(&layout, &fwd);
        assert_eq!(segs_fwd.len(), 4);
        assert!(segs_fwd[0].1.x > segs_fwd[0].0.x);

        // Reversed path s3 -> s0: rises from 0 mm at s3's outer end to 80 mm at s0's outer end
        let rev = resolve_ramp_chain(&layout, &empty_sel, Some("s3"), Some("s0"));
        assert_eq!(
            rev,
            vec![
                ("s3".into(), true),
                ("s2".into(), true),
                ("s1".into(), true),
                ("s0".into(), true),
            ]
        );
        let segs_rev = ramp_chain_direction_segments(&layout, &rev);
        assert!(segs_rev[0].1.x < segs_rev[0].0.x);

        let sum = apply_auto_ramp_for_chain(&mut layout, &catalogs, &rev, 0.0, 80.0, RampProfile::Linear)
            .unwrap();
        assert_eq!(sum.piece_count, 4);
        // s3 entered at c[1] (0 mm) -> exited at c[0] (20 mm); s0 exited at c[0] (80 mm)
        assert!((layout.pieces[3].elevation.unwrap().end - 0.0).abs() < 0.02);
        assert!((layout.pieces[0].elevation.unwrap().start - 80.0).abs() < 0.02);
    }

    #[test]
    fn auto_ramp_breakdown_and_max_grade_end_height() {
        use railbench_core::types::ConnectorRef;
        let cats = builtin_catalogs();
        let e24188 = crate::catalog::find_entry(&cats, "24188").expect("24188");
        let e24130 = crate::catalog::find_entry(&cats, "24130").expect("24130");
        let e24130_r = crate::catalog::find_entry(&cats, "24130").expect("24130");
        let mut p_start = create_piece("a".into(), e24188, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        p_start.elevation = Some(Elevation { start: 0.0, end: 0.0 });
        let mut c1 = create_piece("c1".into(), e24130, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, None);
        // Curve opposite turn after c1 to create an S-curve pair
        let mut c2 = create_piece("c2".into(), e24130_r, Transform { x: 376.6, y: 0.0, rotation: 0.0 }, None);
        c1.connectors[0].connected_to = Some(ConnectorRef { piece_id: "a".into(), connector_index: 1 });
        p_start.connectors[1].connected_to = Some(ConnectorRef { piece_id: "c1".into(), connector_index: 0 });
        c1.connectors[1].connected_to = Some(ConnectorRef { piece_id: "c2".into(), connector_index: 0 });
        c2.connectors[0].connected_to = Some(ConnectorRef { piece_id: "c1".into(), connector_index: 1 });

        let mut p_end = create_piece("b".into(), e24188, Transform { x: 564.9, y: 0.0, rotation: 0.0 }, None);
        p_end.elevation = Some(Elevation { start: 40.0, end: 40.0 });
        c2.connectors[1].connected_to = Some(ConnectorRef { piece_id: "b".into(), connector_index: 0 });
        p_end.connectors[0].connected_to = Some(ConnectorRef { piece_id: "c2".into(), connector_index: 1 });

        let layout = Layout {
            name: "r1".into(),
            scale: "H0".into(),
            pieces: vec![p_start, c1, c2, p_end],
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
            baseboard: None,
            baseboards: None,
            terrain_field: None,
        };
        let mut sel = HashSet::new();
        sel.insert("c1".into());
        sel.insert("c2".into());
        let auto = calculate_auto_ramp(&layout, &sel, &cats).expect("auto ramp inferred");
        assert_eq!(auto.0.len(), 2);
        assert!((auto.1 - 0.0).abs() < 1e-6);
        assert!((auto.2 - 40.0).abs() < 1e-6);

        let rows = build_ramp_breakdown(&layout, &cats, &auto.0, 0.0, 40.0, RampProfile::Linear);
        assert_eq!(rows.len(), 2);
        let max_end = calculate_end_height_from_max_gradient(&layout, &cats, &auto.0, 0.0, 3.0, true, RampProfile::Eased);
        assert!(max_end > 0.0 && max_end < 40.0);
    }
}
