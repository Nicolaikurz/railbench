use super::*;

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
        is_continuous: is_chain_continuous(layout, chain),
    })
}

pub(super) fn piece_curve_sign(piece: &TrackPiece, rev: bool, catalogs: &[Catalog]) -> i8 {
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

#[cfg_attr(not(test), allow(dead_code))]
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

