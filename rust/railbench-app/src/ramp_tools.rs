use std::collections::{HashMap, HashSet};

use railbench_core::geometry::{deg2rad, norm_angle_deg, rotate};
use railbench_core::ramp::compute_ramp_chain_elevations;
use railbench_core::snapping::update_layout_connections;
use railbench_core::types::{Catalog, CatalogEntry, Elevation, Layout, TrackPiece, Transform, Vec2};

use crate::catalog::{create_piece, disconnect_cross_type_connections, find_entry, local_connectors};

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
pub struct RampSummary {
    pub piece_count: usize,
    pub total_length_mm: f64,
    pub total_rise_mm: f64,
    pub avg_gradient_pct: f64,
    pub max_gradient_pct: f64,
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

pub fn preview_auto_ramp(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    start_h: f64,
    end_h: f64,
    profile: RampProfile,
) -> Option<RampSummary> {
    let chain = order_selected_chain(layout, selected);
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

    Some(RampSummary {
        piece_count: lengths.len(),
        total_length_mm: (total_len * 10.0).round() / 10.0,
        total_rise_mm: (total_rise * 10.0).round() / 10.0,
        avg_gradient_pct: (avg_grade * 10.0).round() / 10.0,
        max_gradient_pct: (max_grade * 10.0).round() / 10.0,
    })
}

pub fn apply_auto_ramp(
    layout: &mut Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    start_h: f64,
    end_h: f64,
    profile: RampProfile,
) -> Result<RampSummary, String> {
    let chain = order_selected_chain(layout, selected);
    if chain.is_empty() {
        return Err("Select at least one track piece first.".into());
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

    preview_auto_ramp(layout, catalogs, selected, start_h, end_h, profile)
        .ok_or_else(|| "Unable to compute ramp.".into())
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
    let anchor = layout
        .pieces
        .iter()
        .find(|p| p.id == anchor_id)
        .cloned()
        .ok_or_else(|| "Select one anchor track first.".to_string())?;

    // Find first open connector on anchor (prefer exit connector if multiple are open)
    let mut open_idx: Option<usize> = None;
    for (ci, c) in anchor.connectors.iter().enumerate().rev() {
        let connected = c.connected_to.is_some()
            || layout.pieces.iter().any(|other| {
                other.id != anchor.id
                    && other
                        .connectors
                        .iter()
                        .any(|oc| (c.x - oc.x).hypot(c.y - oc.y) <= 6.0)
            });
        if !connected {
            open_idx = Some(ci);
            break;
        }
    }
    let chosen_ci = open_idx.ok_or_else(|| "Selected anchor track has no open connector.".to_string())?;
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
            let min_curves = (180.0 / c_ang).round().max(2.0) as usize;
            let count = ((min_req_len / c_len).ceil() as usize).max(min_curves).clamp(2, 36);
            for _ in 0..count {
                sequence.push((c_e, curve_left));
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
    let elevs = compute_ramp_chain_elevations(&lengths, &left_flags, h_start, rise, anchor_layer_h);
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
        let mut piece = create_piece(id.clone(), entry, tr, anchor.layer_id.clone());
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
}
