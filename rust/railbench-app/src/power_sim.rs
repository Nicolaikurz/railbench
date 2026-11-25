use std::collections::{BTreeSet, HashMap, HashSet};

use railbench_core::geometry::{deg2rad, norm_angle_deg};
use railbench_core::simulation::{solve_power_dijkstra, solve_short_circuit_dsu};
use railbench_core::types::{Catalog, CatalogEntry, Layout, TrackPiece};

use crate::catalog::{find_entry, is_road_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerStatus {
    Optimal,
    Warning,
    Critical,
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct PiecePowerResult {
    pub voltage: f64,
    pub drop_v: f64,
    pub drop_pct: f64,
    pub status: PowerStatus,
    pub resistance_ohm: f64,
    pub dist_m: f64,
    pub is_feeder: bool,
    pub is_short_circuit: bool,
    pub has_catenary: bool,
    pub catenary_voltage: Option<f64>,
    pub catenary_status: Option<PowerStatus>,
}

#[derive(Debug, Clone)]
pub struct ShortCircuitPoint {
    pub piece_id_1: String,
    pub connector_index_1: usize,
    pub piece_id_2: String,
    pub connector_index_2: usize,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct PowerSimResult {
    pub results: HashMap<String, PiecePowerResult>,
    pub has_short_circuit: bool,
    pub short_circuit_points: Vec<ShortCircuitPoint>,
    pub is_two_wire: bool,
    pub feed_voltage: f64,
    pub total_current_a: f64,
    pub feeder_count: usize,
    pub isolator_count: usize,
    pub optimal_count: usize,
    pub warning_count: usize,
    pub critical_count: usize,
    pub disconnected_count: usize,
    pub min_voltage: f64,
    pub max_drop_pct: f64,
    pub recommendation: String,
    pub turnout_lantern_count: usize,
    pub turnout_decoder_count: usize,
    pub turnout_current_a: f64,
    pub catenary_piece_count: usize,
    pub catenary_feeder_count: usize,
    pub catenary_min_voltage: f64,
    pub catenary_max_drop_pct: f64,
    pub catenary_recommendation: String,
}

fn default_feed_voltage(scale: &str) -> f64 {
    match scale.trim().to_uppercase().as_str() {
        "Z" => 10.0,
        "N" => 12.0,
        "TT" | "H0M" => 14.0,
        "1" => 18.0,
        "G" => 24.0,
        _ => 16.0,
    }
}

fn default_loco_current_ma(scale: &str) -> f64 {
    match scale.trim().to_uppercase().as_str() {
        "Z" | "N" => 100.0,
        "TT" => 300.0,
        "H0" => 450.0,
        "0" => 600.0,
        "1" => 800.0,
        "G" => 1000.0,
        _ => 450.0,
    }
}

fn is_turnout_entry(entry: Option<&CatalogEntry>) -> bool {
    let Some(e) = entry else { return false };
    matches!(
        e.kind.as_str(),
        "turnout-left" | "turnout-right" | "curved-turnout-left" | "curved-turnout-right" | "three-way" | "double-slip"
    )
}

fn is_feeder_piece(piece: &TrackPiece, entry: Option<&CatalogEntry>) -> bool {
    if piece.power_feed == Some(true) {
        return true;
    }
    if piece.catalog_id == "24088" {
        return true;
    }
    entry.map_or(false, crate::catalog::is_feeder_entry)
}

fn piece_length_meters(piece: &TrackPiece, entry: Option<&CatalogEntry>) -> f64 {
    if let Some(cl) = piece.custom_length {
        if cl > 1.0 {
            return cl / 1000.0;
        }
    }
    if let Some(e) = entry {
        if let Some(len) = e.length {
            if len > 1.0 {
                return len / 1000.0;
            }
        }
        if let (Some(r), Some(a)) = (e.radius, e.angle) {
            let meters = (r * deg2rad(a.abs())) / 1000.0;
            if meters > 0.005 {
                return meters;
            }
        }
    }
    if piece.connectors.len() >= 2 {
        let d = (piece.connectors[0].x - piece.connectors[1].x)
            .hypot(piece.connectors[0].y - piece.connectors[1].y)
            / 1000.0;
        if d > 0.01 {
            return d;
        }
    }
    0.188
}

fn rail_terminal(piece_id: &str, entry: Option<&CatalogEntry>, c_idx: usize, side_left: bool) -> String {
    if entry.map_or(false, |e| e.kind == "crossing") {
        if c_idx < 2 {
            let is_rail1 = if c_idx == 0 { side_left } else { !side_left };
            return if is_rail1 {
                format!("{piece_id}:1")
            } else {
                format!("{piece_id}:2")
            };
        } else {
            let is_rail3 = if c_idx == 2 { side_left } else { !side_left };
            return if is_rail3 {
                format!("{piece_id}:3")
            } else {
                format!("{piece_id}:4")
            };
        }
    }
    let is_rail1 = if c_idx == 0 { side_left } else { !side_left };
    if is_rail1 {
        format!("{piece_id}:1")
    } else {
        format!("{piece_id}:2")
    }
}

fn is_two_wire_system(rail_pieces: &[&TrackPiece], catalogs: &[Catalog]) -> bool {
    !rail_pieces
        .iter()
        .any(|p| crate::catalog::has_three_rail_catalog(catalogs, &p.catalog_id))
}

#[derive(Clone, Copy)]
struct JointPair {
    i: usize,
    c1: usize,
    j: usize,
    c2: usize,
    x: f64,
    y: f64,
}

fn collect_non_isolated_joints(
    pieces: &[&TrackPiece],
    id_to_idx: &HashMap<&str, usize>,
    check_catenary_isolator: bool,
) -> Vec<JointPair> {
    let mut visited: HashSet<(usize, usize, usize, usize)> = HashSet::new();
    let mut joints = Vec::new();
    let mut has_unconnected = false;

    let is_iso = |p: &TrackPiece, c_idx: usize| -> bool {
        if check_catenary_isolator {
            p.catenary_isolator == Some(true) && p.catenary_isolated_connector_index.unwrap_or(0) == c_idx
        } else {
            p.is_isolator == Some(true) && p.isolated_connector_index.unwrap_or(0) == c_idx
        }
    };

    for (i, p1) in pieces.iter().enumerate() {
        for (c1_idx, c1) in p1.connectors.iter().enumerate() {
            if let Some(ref conn_to) = c1.connected_to {
                if let Some(&j) = id_to_idx.get(conn_to.piece_id.as_str()) {
                    if i == j {
                        continue;
                    }
                    let c2_idx = conn_to.connector_index;
                    let p2 = pieces[j];
                    if c2_idx < p2.connectors.len() {
                        let c2 = &p2.connectors[c2_idx];
                        let key = if i < j { (i, c1_idx, j, c2_idx) } else { (j, c2_idx, i, c1_idx) };
                        if visited.insert(key) && !is_iso(p1, c1_idx) && !is_iso(p2, c2_idx) {
                            joints.push(JointPair {
                                i,
                                c1: c1_idx,
                                j,
                                c2: c2_idx,
                                x: (c1.x + c2.x) * 0.5,
                                y: (c1.y + c2.y) * 0.5,
                            });
                        }
                    }
                }
            } else {
                has_unconnected = true;
            }
        }
    }

    if has_unconnected {
        const CELL_SIZE: f64 = 12.0;
        const INV_CELL: f64 = 1.0 / CELL_SIZE;
        const MAX_DIST_SQ: f64 = 100.0; // 10.0^2

        let mut grid: HashMap<(i32, i32), Vec<(usize, usize)>> = HashMap::with_capacity(pieces.len() * 2);
        for (j, p2) in pieces.iter().enumerate() {
            for (c2_idx, c2) in p2.connectors.iter().enumerate() {
                let gx = (c2.x * INV_CELL).floor() as i32;
                let gy = (c2.y * INV_CELL).floor() as i32;
                grid.entry((gx, gy)).or_default().push((j, c2_idx));
            }
        }

        for (i, p1) in pieces.iter().enumerate() {
            for (c1_idx, c1) in p1.connectors.iter().enumerate() {
                if c1.connected_to.is_some() {
                    continue;
                }
                let gx = (c1.x * INV_CELL).floor() as i32;
                let gy = (c1.y * INV_CELL).floor() as i32;
                for ox in -1..=1 {
                    for oy in -1..=1 {
                        let Some(bucket) = grid.get(&(gx + ox, gy + oy)) else {
                            continue;
                        };
                        for &(j, c2_idx) in bucket {
                            if j <= i {
                                continue;
                            }
                            let p2 = pieces[j];
                            let c2 = &p2.connectors[c2_idx];
                            let dx = c1.x - c2.x;
                            let dy = c1.y - c2.y;
                            if dx * dx + dy * dy > MAX_DIST_SQ {
                                continue;
                            }
                            let diff_a = (norm_angle_deg(c1.angle - c2.angle) - 180.0).abs();
                            if diff_a <= 35.0 {
                                let key = (i, c1_idx, j, c2_idx);
                                if visited.insert(key) && !is_iso(p1, c1_idx) && !is_iso(p2, c2_idx) {
                                    joints.push(JointPair {
                                        i,
                                        c1: c1_idx,
                                        j,
                                        c2: c2_idx,
                                        x: (c1.x + c2.x) * 0.5,
                                        y: (c1.y + c2.y) * 0.5,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    joints
}

fn build_csr_from_joints(node_count: usize, joints: &[JointPair]) -> (Vec<u32>, Vec<u32>) {
    let mut adj: Vec<BTreeSet<u32>> = vec![BTreeSet::new(); node_count];
    for jt in joints {
        adj[jt.i].insert(jt.j as u32);
        adj[jt.j].insert(jt.i as u32);
    }
    let mut offsets = Vec::with_capacity(node_count + 1);
    let mut neighbors = Vec::with_capacity(joints.len() * 2);
    offsets.push(0);
    for set in &adj {
        for &nbr in set {
            neighbors.push(nbr);
        }
        offsets.push(neighbors.len() as u32);
    }
    (offsets, neighbors)
}

pub fn simulate_layout_power(layout: &Layout, catalogs: &[Catalog]) -> PowerSimResult {
    let feed_voltage = layout
        .feed_voltage
        .unwrap_or_else(|| default_feed_voltage(&layout.scale))
        .clamp(4.0, 30.0);
    let traction_amps = if layout.loco_current_ma.is_some() || layout.loco_count.is_some() {
        let loco_current_ma = layout
            .loco_current_ma
            .unwrap_or_else(|| default_loco_current_ma(&layout.scale))
            .clamp(50.0, 1000.0);
        let loco_count = layout.loco_count.unwrap_or(1).clamp(1, 20);
        (loco_current_ma * loco_count as f64) / 1000.0
    } else if let Some(amps) = layout.train_current_amps.filter(|&a| a > 0.0) {
        amps
    } else {
        (default_loco_current_ma(&layout.scale)) / 1000.0
    };
    let lantern_ma = layout.turnout_lantern_current_ma.unwrap_or(20.0);
    let decoder_ma = layout.turnout_decoder_current_ma.unwrap_or(15.0);

    let rail_pieces: Vec<&TrackPiece> = layout
        .pieces
        .iter()
        .filter(|p| !is_road_id(catalogs, &p.catalog_id))
        .collect();

    let n = rail_pieces.len();
    if n == 0 {
        return PowerSimResult {
            results: HashMap::new(),
            has_short_circuit: false,
            short_circuit_points: Vec::new(),
            is_two_wire: true,
            feed_voltage,
            total_current_a: traction_amps,
            feeder_count: 0,
            isolator_count: 0,
            optimal_count: 0,
            warning_count: 0,
            critical_count: 0,
            disconnected_count: 0,
            min_voltage: feed_voltage,
            max_drop_pct: 0.0,
            recommendation: "Keine Gleise im Plan vorhanden / No tracks in layout.".into(),
            turnout_lantern_count: 0,
            turnout_decoder_count: 0,
            turnout_current_a: 0.0,
            catenary_piece_count: 0,
            catenary_feeder_count: 0,
            catenary_min_voltage: feed_voltage,
            catenary_max_drop_pct: 0.0,
            catenary_recommendation: String::new(),
        };
    }

    let mut id_to_idx: HashMap<&str, usize> = HashMap::with_capacity(n);
    let mut entries: Vec<Option<&CatalogEntry>> = Vec::with_capacity(n);
    let mut lens_m: Vec<f64> = Vec::with_capacity(n);
    let mut feeder_indices: Vec<u32> = Vec::new();
    let mut is_feeder_flags: Vec<bool> = vec![false; n];
    let mut piece_acc_amps: Vec<f64> = vec![0.0; n];
    let mut isolator_count = 0usize;
    let mut turnout_lantern_count = 0usize;
    let mut turnout_decoder_count = 0usize;
    let mut total_acc_ma = 0.0_f64;

    for (i, p) in rail_pieces.iter().enumerate() {
        id_to_idx.insert(p.id.as_str(), i);
        let entry = find_entry(catalogs, &p.catalog_id);
        entries.push(entry);
        lens_m.push(piece_length_meters(p, entry));

        if is_feeder_piece(p, entry) {
            feeder_indices.push(i as u32);
            is_feeder_flags[i] = true;
        }
        if p.is_isolator == Some(true) {
            isolator_count += 1;
        }
        if is_turnout_entry(entry) {
            if let Some(tma) = p.turnout_current_ma.filter(|&v| v > 0.0) {
                total_acc_ma += tma;
                piece_acc_amps[i] = tma / 1000.0;
            } else {
                let mut local_ma = 0.0;
                if p.has_lantern == Some(true) {
                    turnout_lantern_count += 1;
                    local_ma += lantern_ma;
                }
                if p.has_decoder == Some(true) {
                    turnout_decoder_count += 1;
                    local_ma += decoder_ma;
                }
                total_acc_ma += local_ma;
                piece_acc_amps[i] = local_ma / 1000.0;
            }
        }
    }

    let turnout_current_a = total_acc_ma / 1000.0;
    let total_current_a = traction_amps + turnout_current_a;
    let feeder_count = feeder_indices.len();

    let cat_pieces: Vec<&TrackPiece> = rail_pieces
        .iter()
        .copied()
        .filter(|p| p.has_catenary == Some(true))
        .collect();
    let parallel_cat = layout.catenary_circuit_mode.as_deref() == Some("parallel");
    let cat_feed_voltage = if parallel_cat {
        feed_voltage
    } else {
        layout.catenary_feed_voltage.unwrap_or(feed_voltage)
    };
    let is_two_wire = is_two_wire_system(&rail_pieces, catalogs);

    let (
        (solved, short_circuit_pieces, short_circuit_points, rail_joints),
        (cat_min_res, cat_feeder_ids),
    ) = std::thread::scope(|s| {
        let cat_handle = if !cat_pieces.is_empty() {
            let cat_pieces_ref = &cat_pieces;
            Some(s.spawn(move || {
                let mut cat_min_res: HashMap<String, f64> = HashMap::with_capacity(cat_pieces_ref.len());
                let mut cat_feeder_ids: HashSet<String> = HashSet::new();
                let mut cat_id_to_idx: HashMap<&str, usize> = HashMap::with_capacity(cat_pieces_ref.len());
                let mut cat_lens_m: Vec<f64> = Vec::with_capacity(cat_pieces_ref.len());
                let mut cat_feeders: Vec<u32> = Vec::new();
                for (ci, p) in cat_pieces_ref.iter().enumerate() {
                    cat_id_to_idx.insert(p.id.as_str(), ci);
                    let e = find_entry(catalogs, &p.catalog_id);
                    cat_lens_m.push(piece_length_meters(p, e));
                    let is_cf = p.catenary_power_feed == Some(true)
                        || (parallel_cat && is_feeder_piece(p, e));
                    if is_cf {
                        cat_feeders.push(ci as u32);
                        cat_feeder_ids.insert(p.id.clone());
                    }
                }
                let cat_joints = collect_non_isolated_joints(cat_pieces_ref, &cat_id_to_idx, true);
                let (c_offs, c_nbrs) = build_csr_from_joints(cat_pieces_ref.len(), &cat_joints);
                let cat_solved = solve_power_dijkstra(
                    cat_pieces_ref.len(),
                    &c_offs,
                    &c_nbrs,
                    &cat_lens_m,
                    &cat_feeders,
                    0.035,
                    0.02,
                );
                for (ci, p) in cat_pieces_ref.iter().enumerate() {
                    cat_min_res.insert(p.id.clone(), cat_solved[ci]);
                }
                (cat_min_res, cat_feeder_ids)
            }))
        } else {
            None
        };

        let rail_joints = collect_non_isolated_joints(&rail_pieces, &id_to_idx, false);
        let rail_joints_ref = &rail_joints;
        let rail_pieces_ref = &rail_pieces;
        let entries_ref = &entries;
        let lens_m_ref = &lens_m;
        let feeder_indices_ref = &feeder_indices;

        let (solved, (short_circuit_pieces, short_circuit_points)) = std::thread::scope(|s_inner| {
            let dsu_handle = if is_two_wire && !rail_joints_ref.is_empty() {
                Some(s_inner.spawn(move || {
                    let mut short_pieces: HashSet<String> = HashSet::new();
                    let mut short_points: Vec<ShortCircuitPoint> = Vec::new();
                    let mut term_map: HashMap<String, u32> = HashMap::new();
                    let mut get_term = |key: String| -> u32 {
                        if let Some(&id) = term_map.get(&key) {
                            id
                        } else {
                            let id = term_map.len() as u32;
                            term_map.insert(key, id);
                            id
                        }
                    };

                    let mut edges: Vec<u32> = Vec::with_capacity(rail_joints_ref.len() * 4);
                    for jt in rail_joints_ref {
                        let p1 = rail_pieces_ref[jt.i];
                        let p2 = rail_pieces_ref[jt.j];
                        let e1 = entries_ref[jt.i];
                        let e2 = entries_ref[jt.j];
                        edges.push(get_term(rail_terminal(&p1.id, e1, jt.c1, true)));
                        edges.push(get_term(rail_terminal(&p1.id, e1, jt.c1, false)));
                        edges.push(get_term(rail_terminal(&p2.id, e2, jt.c2, true)));
                        edges.push(get_term(rail_terminal(&p2.id, e2, jt.c2, false)));
                    }

                    let mut pair_piece_ids: Vec<&str> = Vec::with_capacity(n + 8);
                    let mut rail_pairs: Vec<u32> = Vec::with_capacity(n * 2 + 16);
                    for (i, p) in rail_pieces_ref.iter().enumerate() {
                        pair_piece_ids.push(p.id.as_str());
                        rail_pairs.push(get_term(format!("{}:1", p.id)));
                        rail_pairs.push(get_term(format!("{}:2", p.id)));
                        if entries_ref[i].map_or(false, |e| e.kind == "crossing") {
                            pair_piece_ids.push(p.id.as_str());
                            rail_pairs.push(get_term(format!("{}:3", p.id)));
                            rail_pairs.push(get_term(format!("{}:4", p.id)));
                        }
                    }

                    let dsu_out = solve_short_circuit_dsu(term_map.len(), &edges, &rail_pairs);
                    for (k, jt) in rail_joints_ref.iter().enumerate() {
                        if dsu_out.get(k).copied().unwrap_or(0) != 0 {
                            short_points.push(ShortCircuitPoint {
                                piece_id_1: rail_pieces_ref[jt.i].id.clone(),
                                connector_index_1: jt.c1,
                                piece_id_2: rail_pieces_ref[jt.j].id.clone(),
                                connector_index_2: jt.c2,
                                x: jt.x,
                                y: jt.y,
                            });
                        }
                    }
                    for (k, &pid) in pair_piece_ids.iter().enumerate() {
                        if dsu_out.get(rail_joints_ref.len() + k).copied().unwrap_or(0) != 0 {
                            short_pieces.insert(pid.to_string());
                        }
                    }
                    (short_pieces, short_points)
                }))
            } else {
                None
            };

            let (offsets, neighbors) = build_csr_from_joints(n, rail_joints_ref);
            let solved = solve_power_dijkstra(n, &offsets, &neighbors, lens_m_ref, feeder_indices_ref, 0.08, 0.05);
            let dsu_res = dsu_handle
                .and_then(|h| h.join().ok())
                .unwrap_or_else(|| (HashSet::new(), Vec::new()));
            (solved, dsu_res)
        });

        let cat_res = cat_handle
            .and_then(|h| h.join().ok())
            .unwrap_or_else(|| (HashMap::new(), HashSet::new()));
        ((solved, short_circuit_pieces, short_circuit_points, rail_joints), cat_res)
    });

    // Compute per-branch subtree accessory load along shortest-path feeder tree
    let mut subtree_load_amps = piece_acc_amps.clone();
    if feeder_count > 0 && turnout_current_a > 0.0 {
        let mut adj_list: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for jt in &rail_joints {
            let edge_r = (lens_m[jt.i] * 0.5 + lens_m[jt.j] * 0.5) * 0.08 + 0.05;
            adj_list[jt.i].push((jt.j, edge_r));
            adj_list[jt.j].push((jt.i, edge_r));
        }
        let mut parent: Vec<Option<usize>> = vec![None; n];
        for i in 0..n {
            if is_feeder_flags[i] || !solved[i].is_finite() {
                continue;
            }
            let my_r = solved[i];
            let mut best_p: Option<(usize, f64)> = None;
            for &(nbr, _er) in &adj_list[i] {
                if solved[nbr].is_finite() && solved[nbr] < my_r {
                    if best_p.map_or(true, |(_, br)| solved[nbr] < br) {
                        best_p = Some((nbr, solved[nbr]));
                    }
                }
            }
            if let Some((p_idx, _)) = best_p {
                parent[i] = Some(p_idx);
            }
        }
        let mut order: Vec<usize> = (0..n).filter(|&i| solved[i].is_finite()).collect();
        order.sort_by(|&a, &b| solved[b].partial_cmp(&solved[a]).unwrap_or(std::cmp::Ordering::Equal));
        for idx in order {
            if let Some(p_idx) = parent[idx] {
                let child_load = subtree_load_amps[idx];
                subtree_load_amps[p_idx] += child_load;
            }
        }
    }

    let has_short_circuit = !short_circuit_pieces.is_empty() || !short_circuit_points.is_empty();
    let mut catenary_min_voltage = cat_feed_voltage;
    let mut catenary_max_drop_pct = 0.0_f64;

    let mut results = HashMap::with_capacity(n);
    let mut min_voltage = feed_voltage;
    let mut max_drop_pct = 0.0_f64;
    let mut optimal_count = 0usize;
    let mut warning_count = 0usize;
    let mut critical_count = 0usize;
    let mut disconnected_count = 0usize;

    for (i, p) in rail_pieces.iter().enumerate() {
        let mut res = solved[i];
        let dist_m = solved[n + i];
        let is_feeder = is_feeder_flags[i];
        let is_short = short_circuit_pieces.contains(&p.id);
        let has_cat = p.has_catenary == Some(true);
        let cat_res = cat_min_res.get(&p.id).copied();
        let is_cat_feeder = cat_feeder_ids.contains(&p.id);

        if parallel_cat && has_cat {
            if let Some(cr) = cat_res {
                if res.is_finite() && cr.is_finite() && (res + cr) > 0.0 {
                    res = (res * cr) / (res + cr);
                }
            }
        }

        let (cat_v, cat_st) = if has_cat {
            if is_cat_feeder {
                (Some((cat_feed_voltage * 100.0).round() / 100.0), Some(PowerStatus::Optimal))
            } else {
                let cr = cat_res.unwrap_or(f64::INFINITY);
                if !cr.is_finite() {
                    catenary_min_voltage = 0.0;
                    catenary_max_drop_pct = 100.0;
                    (Some(0.0), Some(PowerStatus::Disconnected))
                } else {
                    let cd = traction_amps * cr;
                    let cv = (cat_feed_voltage - cd).max(0.0);
                    let cpct = (cd / cat_feed_voltage) * 100.0;
                    if cv < catenary_min_voltage {
                        catenary_min_voltage = cv;
                    }
                    if cpct > catenary_max_drop_pct {
                        catenary_max_drop_pct = cpct;
                    }
                    let cst = if cpct > 15.0 {
                        PowerStatus::Critical
                    } else if cpct > 5.0 {
                        PowerStatus::Warning
                    } else {
                        PowerStatus::Optimal
                    };
                    (Some((cv * 100.0).round() / 100.0), Some(cst))
                }
            }
        } else {
            (None, None)
        };

        if !res.is_finite() {
            if feeder_count == 0 {
                min_voltage = 0.0;
                max_drop_pct = 100.0;
            }
            let st = if is_short { PowerStatus::Critical } else { PowerStatus::Disconnected };
            disconnected_count += 1;
            results.insert(
                p.id.clone(),
                PiecePowerResult {
                    voltage: 0.0,
                    drop_v: feed_voltage,
                    drop_pct: 100.0,
                    status: st,
                    resistance_ohm: f64::INFINITY,
                    dist_m: 0.0,
                    is_feeder,
                    is_short_circuit: is_short,
                    has_catenary: has_cat,
                    catenary_voltage: cat_v,
                    catenary_status: cat_st,
                },
            );
            continue;
        }

        let (v, drop_v, drop_pct, status) = if is_short {
            critical_count += 1;
            min_voltage = 0.0;
            max_drop_pct = 100.0;
            (0.0, feed_voltage, 100.0, PowerStatus::Critical)
        } else if is_feeder {
            optimal_count += 1;
            (feed_voltage, 0.0, 0.0, PowerStatus::Optimal)
        } else {
            let eff_current_a = traction_amps + subtree_load_amps[i].min(turnout_current_a);
            let dv = eff_current_a * res;
            let vv = (feed_voltage - dv).max(0.0);
            let pct = (dv / feed_voltage) * 100.0;
            let st = if pct > 15.0 {
                critical_count += 1;
                PowerStatus::Critical
            } else if pct > 5.0 {
                warning_count += 1;
                PowerStatus::Warning
            } else {
                optimal_count += 1;
                PowerStatus::Optimal
            };
            if vv < min_voltage {
                min_voltage = vv;
            }
            if pct > max_drop_pct {
                max_drop_pct = pct;
            }
            (vv, dv, pct, st)
        };

        results.insert(
            p.id.clone(),
            PiecePowerResult {
                voltage: (v * 100.0).round() / 100.0,
                drop_v: (drop_v * 100.0).round() / 100.0,
                drop_pct: (drop_pct * 10.0).round() / 10.0,
                status,
                resistance_ohm: (res * 1000.0).round() / 1000.0,
                dist_m: (dist_m * 100.0).round() / 100.0,
                is_feeder,
                is_short_circuit: is_short,
                has_catenary: has_cat,
                catenary_voltage: cat_v,
                catenary_status: cat_st,
            },
        );
    }

    let recommendation = if has_short_circuit {
        format!(
            "SHORT CIRCUIT detected at {} joint(s)! Place an Isolator (I) on a reversing loop / turnout joint.",
            short_circuit_points.len().max(1)
        )
    } else if feeder_count == 0 {
        "No power feeder placed yet. Select a track and press F to mark a Feeder.".into()
    } else if disconnected_count > 0 {
        format!("{disconnected_count} track(s) are unpowered (isolated or disconnected from any feeder).")
    } else if critical_count > 0 {
        format!("Critical voltage drop (>15%) on {critical_count} track(s). Add another feeder nearby.")
    } else if warning_count > 0 {
        format!("Moderate voltage drop (5–15%) on {warning_count} track(s) at {total_current_a:.2} A load.")
    } else {
        format!("Optimal power across all {optimal_count} tracks ({total_current_a:.2} A load, max drop {max_drop_pct:.1}%).")
    };

    let catenary_piece_count = cat_pieces.len();
    let catenary_feeder_count = cat_feeder_ids.len();
    let catenary_recommendation = if catenary_piece_count == 0 {
        String::new()
    } else if catenary_feeder_count == 0 {
        "Catenary has no feeder yet (Shift+F to add catenary feeder).".into()
    } else if catenary_max_drop_pct > 15.0 {
        "Catenary voltage drop is critical (>15%). Add another catenary feeder.".into()
    } else {
        format!(
            "Catenary OK ({catenary_piece_count} pieces, min {:.2} V, drop {:.1}%).",
            (catenary_min_voltage * 100.0).round() / 100.0,
            (catenary_max_drop_pct * 10.0).round() / 10.0
        )
    };

    PowerSimResult {
        results,
        has_short_circuit,
        short_circuit_points,
        is_two_wire,
        feed_voltage,
        total_current_a,
        feeder_count,
        isolator_count,
        optimal_count,
        warning_count,
        critical_count,
        disconnected_count,
        min_voltage: (min_voltage * 100.0).round() / 100.0,
        max_drop_pct: (max_drop_pct * 10.0).round() / 10.0,
        recommendation,
        turnout_lantern_count,
        turnout_decoder_count,
        turnout_current_a,
        catenary_piece_count,
        catenary_feeder_count,
        catenary_min_voltage: (catenary_min_voltage * 100.0).round() / 100.0,
        catenary_max_drop_pct: (catenary_max_drop_pct * 10.0).round() / 10.0,
        catenary_recommendation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, create_piece, find_entry};
    use railbench_core::snapping::update_layout_connections;
    use railbench_core::types::{Transform};

    fn make_test_layout(pieces: Vec<TrackPiece>) -> Layout {
        Layout {
            name: "Test".into(),
            scale: "H0".into(),
            pieces,
            models: None,
            feed_voltage: Some(16.0),
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: Some(500.0),
            loco_count: Some(2),
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
            terrain_field: None,
        }
    }

    #[test]
    fn voltage_drop_and_isolator_disconnect() {
        let catalogs = builtin_catalogs();
        // Use a 2-wire catalog entry (Roco Line H0 or Piko A H0)
        let entry = find_entry(&catalogs, "24188").unwrap();
        let mut p0 = create_piece("p0".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        p0.power_feed = Some(true);
        let p1 = create_piece("p1".into(), entry, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, None);
        let mut p2 = create_piece("p2".into(), entry, Transform { x: 376.6, y: 0.0, rotation: 0.0 }, None);
        p2.is_isolator = Some(true);
        p2.isolated_connector_index = Some(0);

        let mut layout = make_test_layout(vec![p0, p1, p2]);
        update_layout_connections(&mut layout, 4.0, 35.0, 1.0);
        let sim = simulate_layout_power(&layout, &catalogs);

        assert_eq!(sim.results["p0"].status, PowerStatus::Optimal);
        assert!((sim.results["p0"].voltage - 16.0).abs() < 1e-6);
        assert!(sim.results["p1"].voltage < 16.0 && sim.results["p1"].voltage > 15.0);
        assert_eq!(sim.results["p2"].status, PowerStatus::Disconnected);
        assert_eq!(sim.disconnected_count, 1);
    }

    #[test]
    fn two_wire_reversing_loop_short_circuit() {
        let catalogs = builtin_catalogs();
        // Piko A 55220 is a 2-wire H0 left turnout, 55200 is a 2-wire H0 straight
        let turnout = find_entry(&catalogs, "55220").unwrap();
        let straight = find_entry(&catalogs, "55200").unwrap();
        let mut p0 = create_piece("p0".into(), turnout, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        p0.power_feed = Some(true);
        let mut p1 = create_piece("p1".into(), straight, Transform { x: 250.0, y: 0.0, rotation: 0.0 }, None);
        // Reversing loop: branch c[1] -> p1.c[0], p1.c[1] -> back into turnout's other exit c[2]
        p0.connectors[1].connected_to = Some(railbench_core::types::ConnectorRef { piece_id: "p1".into(), connector_index: 0 });
        p1.connectors[0].connected_to = Some(railbench_core::types::ConnectorRef { piece_id: "p0".into(), connector_index: 1 });
        p1.connectors[1].connected_to = Some(railbench_core::types::ConnectorRef { piece_id: "p0".into(), connector_index: 2 });
        p0.connectors[2].connected_to = Some(railbench_core::types::ConnectorRef { piece_id: "p1".into(), connector_index: 1 });

        let layout = make_test_layout(vec![p0, p1]);
        let sim = simulate_layout_power(&layout, &catalogs);
        assert!(sim.is_two_wire);
        assert!(sim.has_short_circuit);

        // Now isolate connector 1 of p1 -> reversing loop cut off, short circuit clears!
        let mut layout_iso = layout;
        layout_iso.pieces[1].is_isolator = Some(true);
        layout_iso.pieces[1].isolated_connector_index = Some(1);
        let sim2 = simulate_layout_power(&layout_iso, &catalogs);
        assert!(!sim2.has_short_circuit);
    }

    #[test]
    fn spatial_hash_unconnected_joints_propagate_power() {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "55200").unwrap();
        let mut pieces = Vec::new();
        let step = entry.length.unwrap_or(230.9);
        for idx in 0..8 {
            let mut p = create_piece(
                format!("p{idx}"),
                entry,
                Transform { x: idx as f64 * step, y: 0.0, rotation: 0.0 },
                None,
            );
            if idx == 0 {
                p.power_feed = Some(true);
            }
            // Deliberately leave connected_to == None to test the spatial-hash proximity fallback!
            pieces.push(p);
        }
        let layout = make_test_layout(pieces);
        let sim = simulate_layout_power(&layout, &catalogs);
        assert!(sim.is_two_wire);
        assert_eq!(sim.disconnected_count, 0);
        assert!(sim.results["p7"].voltage > 15.0 && sim.results["p7"].voltage < 16.0);
    }

    #[test]
    fn unequipped_turnout_lantern_draws_zero_ma_and_equipped_draws_lantern_ma() {
        let catalogs = builtin_catalogs();
        let turnout = find_entry(&catalogs, "24611").expect("24611 in catalog");
        let mut p0 = create_piece("sw0".into(), turnout, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        p0.power_feed = Some(true);

        let mut layout_unequipped = make_test_layout(vec![p0.clone()]);
        layout_unequipped.loco_count = Some(1);
        layout_unequipped.turnout_lantern_current_ma = Some(25.0);
        let sim_unequipped = simulate_layout_power(&layout_unequipped, &catalogs);
        assert!(
            (sim_unequipped.total_current_a - 0.5).abs() < 1e-9,
            "unequipped turnout lantern should add 0 mA on top of 0.5 A traction, got {} A",
            sim_unequipped.total_current_a
        );

        p0.has_lantern = Some(true);
        let mut layout_equipped = make_test_layout(vec![p0]);
        layout_equipped.loco_count = Some(1);
        layout_equipped.turnout_lantern_current_ma = Some(25.0);
        let sim_equipped = simulate_layout_power(&layout_equipped, &catalogs);
        assert!(
            (sim_equipped.total_current_a - 0.525).abs() < 1e-9,
            "equipped turnout lantern should add 25 mA (0.525 A total), got {} A",
            sim_equipped.total_current_a
        );
        assert_eq!(sim_equipped.turnout_lantern_count, 1);
        assert!((sim_equipped.turnout_current_a - 0.025).abs() < 1e-9);
    }

    #[test]
    fn subtree_drop_catenary_stats_and_feeder_keyword_detection() {
        let catalogs = builtin_catalogs();
        let entry_24088 = find_entry(&catalogs, "24088").expect("24088 feeder");
        assert!(crate::catalog::is_feeder_entry(entry_24088));

        let straight = find_entry(&catalogs, "24188").expect("24188 straight");
        let mut p0 = create_piece("p0".into(), straight, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        p0.power_feed = Some(true);
        p0.has_catenary = Some(true);
        p0.catenary_power_feed = Some(true);
        let mut p1 = create_piece("p1".into(), straight, Transform { x: 188.3, y: 0.0, rotation: 0.0 }, None);
        p1.has_catenary = Some(true);
        let mut layout = make_test_layout(vec![p0, p1]);
        railbench_core::snapping::update_layout_connections(&mut layout, 4.0, 35.0, 1.0);
        layout.feed_voltage = Some(16.0);
        layout.loco_count = Some(2);
        layout.catenary_circuit_mode = Some("separate".into());

        let sim = simulate_layout_power(&layout, &catalogs);
        assert_eq!(sim.catenary_piece_count, 2);
        assert_eq!(sim.catenary_feeder_count, 1);
    }
}
