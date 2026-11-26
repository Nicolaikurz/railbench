use super::*;

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

fn is_piece_layer_editable(layout: &Layout, layer_id: Option<&str>) -> bool {
    let lid = layer_id.unwrap_or("layer-1");
    if let Some(layers) = &layout.layers {
        if let Some(l) = layers.iter().find(|layer| layer.id == lid) {
            return l.visible && !l.locked;
        }
    }
    true
}

/// Collects open connectors across visible, unlocked layers, prioritizing `preferred_piece`
/// and selected tracks first (up to 60 entries).
pub fn collect_open_connector_options(
    layout: &Layout,
    selected: &HashSet<String>,
    preferred_piece: Option<&str>,
) -> Vec<StartConnectorOption> {
    let mut options = Vec::new();
    for p in &layout.pieces {
        if !is_piece_layer_editable(layout, p.layer_id.as_deref()) {
            continue;
        }
        let open_conns = available_open_connectors_for_piece(layout, &p.id);
        let layer_h = layer_height_of(layout, p.layer_id.as_deref());
        let is_sel = selected.contains(&p.id);
        for ci in open_conns {
            let Some(c) = p.connectors.get(ci) else {
                continue;
            };
            let local_z = match p.elevation {
                Some(el) => {
                    if ci == 0 {
                        el.start
                    } else {
                        el.end
                    }
                }
                None => 0.0,
            };
            let z = ((layer_h + local_z) * 10.0).round() / 10.0;
            options.push(StartConnectorOption {
                piece_id: p.id.clone(),
                catalog_id: p.catalog_id.clone(),
                connector_index: ci,
                world_x: c.x,
                world_y: c.y,
                world_z: z,
                angle_deg: norm_angle_deg(c.angle),
                is_selected: is_sel,
            });
        }
    }

    options.sort_by(|a, b| {
        let a_pref = preferred_piece.is_some_and(|pid| pid == a.piece_id);
        let b_pref = preferred_piece.is_some_and(|pid| pid == b.piece_id);
        b_pref
            .cmp(&a_pref)
            .then_with(|| b.is_selected.cmp(&a.is_selected))
            .then_with(|| a.piece_id.cmp(&b.piece_id))
            .then_with(|| a.connector_index.cmp(&b.connector_index))
    });
    options.truncate(60);
    options
}

/// Finds the nearest open connector on `piece_id` itself, or (if none are open) across its
/// connected track component via BFS + 8 mm proximity fallback.
pub fn find_nearest_open_connector_in_chain(
    layout: &Layout,
    piece_id: &str,
    click_world: Vec2,
) -> Option<(String, usize)> {
    let pick_closest = |candidates: Vec<(String, usize, Vec2)>| -> Option<(String, usize)> {
        candidates
            .into_iter()
            .min_by(|a, b| {
                let da = (a.2.x - click_world.x).hypot(a.2.y - click_world.y);
                let db = (b.2.x - click_world.x).hypot(b.2.y - click_world.y);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(pid, ci, _)| (pid, ci))
    };

    if let Some(p) = layout.pieces.iter().find(|pc| pc.id == piece_id) {
        let own_open = available_open_connectors_for_piece(layout, piece_id);
        if !own_open.is_empty() {
            let candidates = own_open
                .into_iter()
                .filter_map(|ci| {
                    p.connectors
                        .get(ci)
                        .map(|c| (p.id.clone(), ci, Vec2 { x: c.x, y: c.y }))
                })
                .collect();
            return pick_closest(candidates);
        }
    }

    let n = layout.pieces.len();
    let Some(start_idx) = layout.pieces.iter().position(|p| p.id == piece_id) else {
        return None;
    };
    let id_to_idx: HashMap<&str, usize> = layout
        .pieces
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();

    let mut visited = vec![false; n];
    let mut queue = VecDeque::new();
    let mut component = Vec::new();
    visited[start_idx] = true;
    queue.push_back(start_idx);

    while let Some(u) = queue.pop_front() {
        component.push(u);
        let pu = &layout.pieces[u];
        for cu in &pu.connectors {
            if let Some(ref link) = cu.connected_to {
                if let Some(&v) = id_to_idx.get(link.piece_id.as_str()) {
                    if !visited[v] {
                        visited[v] = true;
                        queue.push_back(v);
                    }
                }
            }
            for (v, pv) in layout.pieces.iter().enumerate() {
                if visited[v] {
                    continue;
                }
                if pv
                    .connectors
                    .iter()
                    .any(|cv| (cu.x - cv.x).hypot(cu.y - cv.y) <= 8.0)
                {
                    visited[v] = true;
                    queue.push_back(v);
                }
            }
        }
    }

    let mut candidates = Vec::new();
    for idx in component {
        let p = &layout.pieces[idx];
        for ci in available_open_connectors_for_piece(layout, &p.id) {
            if let Some(c) = p.connectors.get(ci) {
                candidates.push((p.id.clone(), ci, Vec2 { x: c.x, y: c.y }));
            }
        }
    }
    pick_closest(candidates)
}

#[allow(clippy::too_many_arguments)]
fn build_ramp_or_helix_pieces(
    layout: &Layout,
    catalogs: &[Catalog],
    active_catalog_idx: usize,
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
    start_seq: u64,
) -> Result<(Vec<TrackPiece>, RampSummary, f64), String> {
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
    let mut actual_turns = 0.0_f64;
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
            let target_arc = arc_angle_deg.clamp(45.0, 360.0);
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
            let by_turns = if turns > 0.0 {
                ((turns * 360.0) / c_ang).ceil() as usize
            } else {
                0
            };
            let by_grade = (min_req_len / c_len).ceil() as usize;
            let count = by_turns.max(by_grade).clamp(6, 72);
            actual_turns = (count as f64 * c_ang) / 360.0;
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
    let mut seq = start_seq;
    let mut pieces = Vec::with_capacity(sequence.len());

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

        let id = format!("ramp-{seq}");
        seq += 1;
        let mut piece = create_piece(id, entry, tr, place_layer_id.clone());
        piece.elevation = Some(Elevation {
            start: elevs[i * 2],
            end: elevs[i * 2 + 1],
        });

        let out_c = &piece.connectors[out_idx.min(piece.connectors.len() - 1)];
        curr_x = out_c.x;
        curr_y = out_c.y;
        curr_ang = out_c.angle;

        pieces.push(piece);
    }

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
        is_continuous: true,
    };
    Ok((pieces, summary, actual_turns))
}

#[allow(clippy::too_many_arguments)]
pub fn preview_generate_ramp_or_helix(
    layout: &Layout,
    catalogs: &[Catalog],
    active_catalog_idx: usize,
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
) -> Result<(Vec<TrackPiece>, RampSummary, f64), String> {
    build_ramp_or_helix_pieces(
        layout,
        catalogs,
        active_catalog_idx,
        anchor_id,
        preferred_conn_idx,
        target_layer_id,
        shape,
        curve_left,
        arc_angle_deg,
        target_height_mm,
        max_grade_pct,
        turns,
        selected_curve_id,
        900_000,
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
    let (pieces, summary, _) = build_ramp_or_helix_pieces(
        layout,
        catalogs,
        active_catalog_idx,
        anchor_id,
        preferred_conn_idx,
        target_layer_id,
        shape,
        curve_left,
        arc_angle_deg,
        target_height_mm,
        max_grade_pct,
        turns,
        selected_curve_id,
        *next_seq,
    )?;
    let new_ids: Vec<String> = pieces.iter().map(|p| p.id.clone()).collect();
    *next_seq += pieces.len() as u64;
    layout.pieces.extend(pieces);

    update_layout_connections(layout, 4.0, 35.0, 2.0);
    disconnect_cross_type_connections(layout, catalogs);

    Ok((new_ids, summary))
}

