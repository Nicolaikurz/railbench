use std::collections::HashMap;

use railbench_core::types::{Catalog, CatalogEntry, Layout, TrackPiece, Vec2};

use crate::catalog::{find_entry, is_road_id};
use crate::export::sample_track_branches_world;
use crate::model_tools::scale_factor_from_h0;

use super::{VehicleDomain, VehicleInstance, VehicleRenderPose};

pub(super) struct CachedPieceTrack<'a> {
    pub(super) entry: Option<&'a CatalogEntry>,
    pub(super) is_special: bool,
    pub(super) branches: Vec<Vec<Vec2>>,
    pub(super) lengths: Vec<f64>,
    pub(super) layer_h: f64,
    pub(super) is_road: bool,
}

/// Per-step spatial/polyline cache so vehicle physics, collision lookahead probes,
/// and coupled-consist bogie walks share sampled track geometry in O(1).
pub(super) struct TrackWalkCache<'a> {
    layout: &'a Layout,
    catalogs: &'a [Catalog],
    tt_angles: &'a HashMap<String, f64>,
    tr_offsets: &'a HashMap<String, f64>,
    piece_by_id: HashMap<&'a str, usize>,
    layer_heights: HashMap<&'a str, f64>,
    piece_cache: HashMap<usize, CachedPieceTrack<'a>>,
}

impl<'a> TrackWalkCache<'a> {
    pub(super) fn new(
        layout: &'a Layout,
        catalogs: &'a [Catalog],
        tt_angles: &'a HashMap<String, f64>,
        tr_offsets: &'a HashMap<String, f64>,
    ) -> Self {
        let mut piece_by_id = HashMap::with_capacity(layout.pieces.len());
        for (i, p) in layout.pieces.iter().enumerate() {
            piece_by_id.insert(p.id.as_str(), i);
        }
        let mut layer_heights = HashMap::new();
        if let Some(layers) = &layout.layers {
            layer_heights.reserve(layers.len());
            for l in layers {
                if let Some(h) = l.height_mm {
                    layer_heights.insert(l.id.as_str(), h);
                }
            }
        }
        Self {
            layout,
            catalogs,
            tt_angles,
            tr_offsets,
            piece_by_id,
            layer_heights,
            piece_cache: HashMap::with_capacity(32),
        }
    }

    #[inline]
    pub(super) fn piece_idx(&self, pid: &str) -> Option<usize> {
        self.piece_by_id.get(pid).copied()
    }

    #[inline]
    pub(super) fn piece(&self, idx: usize) -> &'a TrackPiece {
        &self.layout.pieces[idx]
    }

    pub(super) fn get_or_sample(&mut self, idx: usize) -> &CachedPieceTrack<'a> {
        if !self.piece_cache.contains_key(&idx) {
            let piece = &self.layout.pieces[idx];
            let entry = find_entry(self.catalogs, &piece.catalog_id);
            let is_special = entry
                .is_some_and(|e| matches!(e.kind.as_str(), "turntable" | "transfer-table"));
            let branches =
                sample_active_piece_branches(piece, entry, self.tt_angles, self.tr_offsets);
            let lengths = branches.iter().map(|b| polyline_length(b)).collect();
            let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
            let layer_h = self.layer_heights.get(lid).copied().unwrap_or(0.0);
            let is_road = is_road_id(self.catalogs, &piece.catalog_id);
            self.piece_cache.insert(
                idx,
                CachedPieceTrack {
                    entry,
                    is_special,
                    branches,
                    lengths,
                    layer_h,
                    is_road,
                },
            );
        }
        &self.piece_cache[&idx]
    }

    pub(super) fn find_connected_next(
        &mut self,
        piece_idx: usize,
        exit_conn: usize,
        domain: VehicleDomain,
    ) -> Option<(usize, usize)> {
        let want_road = domain == VehicleDomain::Road;
        let cur_lh = self.get_or_sample(piece_idx).layer_h;
        let cur_piece = &self.layout.pieces[piece_idx];
        let c = cur_piece.connectors.get(exit_conn)?;
        let cur_z = cur_lh
            + cur_piece
                .elevation
                .map_or(0.0, |el| if exit_conn == 0 { el.start } else { el.end });
        let cur_ang = c.angle;
        let (cx, cy) = (c.x, c.y);

        if let Some(ref link) = c.connected_to {
            if let Some(next_idx) = self.piece_idx(&link.piece_id) {
                let cached = self.get_or_sample(next_idx);
                if cached.is_road == want_road && !(want_road && cached.is_special) {
                    return Some((next_idx, link.connector_index));
                }
            }
        }
        let mut best: Option<(f64, usize, usize)> = None;
        for (oi, _other) in self.layout.pieces.iter().enumerate() {
            if oi == piece_idx {
                continue;
            }
            let cached = self.get_or_sample(oi);
            if cached.is_road != want_road || (want_road && cached.is_special) {
                continue;
            }
            let olh = cached.layer_h;
            let other = &self.layout.pieces[oi];
            for (ci, oc) in other.connectors.iter().enumerate() {
                let d = (oc.x - cx).hypot(oc.y - cy);
                if d >= 5.0 {
                    continue;
                }
                let oz = olh
                    + other
                        .elevation
                        .map_or(0.0, |el| if ci == 0 { el.start } else { el.end });
                if (oz - cur_z).abs() > 12.0 {
                    continue;
                }
                let diff_a = (railbench_core::geometry::norm_angle_deg(oc.angle - cur_ang) - 180.0).abs();
                if diff_a > 45.0 {
                    continue;
                }
                if best.as_ref().is_none_or(|(bd, _, _)| d < *bd) {
                    best = Some((d, oi, ci));
                }
            }
        }
        best.map(|(_, ni, ci)| (ni, ci))
    }

    pub(super) fn track_scale_and_gauge(&mut self, piece_id: &str, domain: VehicleDomain) -> (f64, f64) {
        if domain == VehicleDomain::Rail {
            if let Some(idx) = self.piece_idx(piece_id) {
                let cached = self.get_or_sample(idx);
                if let Some(g) = cached.entry.and_then(|e| e.gauge) {
                    if g > 2.0 {
                        return ((g / 16.5).clamp(0.35, 3.0), g);
                    }
                }
            }
        }
        let sf = scale_factor_from_h0(&self.layout.scale).max(0.1);
        (sf, 16.5 * sf)
    }

    pub(super) fn resolve_transition(
        &mut self,
        turnout_states: &HashMap<String, usize>,
        path_history: &[(String, usize)],
        next_idx: usize,
        entry_conn: usize,
        use_history: bool,
    ) -> Result<(usize, usize, f64, bool, Option<usize>), ()> {
        let next_piece = &self.layout.pieces[next_idx];
        let next_pid = next_piece.id.as_str();
        let hist_br = if use_history {
            path_history
                .iter()
                .rev()
                .find(|(pid, _)| pid == next_pid)
                .map(|(_, b)| *b)
        } else {
            None
        };
        let sw = turnout_states.get(next_pid).copied().unwrap_or(0);
        let (is_special, entry, br_count, br_len_fn_data) = {
            let cached = self.get_or_sample(next_idx);
            (cached.is_special, cached.entry, cached.branches.len(), cached.lengths.clone())
        };
        if is_special {
            if let Some((at_start, blen)) = check_special_entry(
                next_piece,
                entry,
                entry_conn,
                self.tt_angles,
                self.tr_offsets,
            ) {
                return Ok((
                    next_idx,
                    0,
                    if at_start { 0.0 } else { blen },
                    at_start,
                    None,
                ));
            }
            return Err(());
        }
        let (br, at_start, auto_st) =
            resolve_piece_entry(next_piece, entry, entry_conn, sw, br_count, hist_br);
        let dist = if at_start {
            0.0
        } else {
            br_len_fn_data.get(br).copied().unwrap_or(0.0)
        };
        Ok((next_idx, br, dist, at_start, auto_st))
    }
}

pub(super) fn norm_deg_360(a: f64) -> f64 {
    ((a % 360.0) + 360.0) % 360.0
}

pub(super) fn shortest_angle_diff_deg(from: f64, to: f64) -> f64 {
    let mut d = norm_deg_360(to) - norm_deg_360(from);
    if d > 180.0 {
        d -= 360.0;
    } else if d < -180.0 {
        d += 360.0;
    }
    d
}

pub fn turntable_slot_angles(entry: Option<&CatalogEntry>) -> Vec<f64> {
    entry
        .and_then(|e| e.local_connectors.as_ref())
        .filter(|lc| !lc.is_empty())
        .map(|lc| lc.iter().map(|c| norm_deg_360(c.angle)).collect())
        .unwrap_or_else(|| (0..30).map(|i| i as f64 * 12.0).collect())
}

pub(super) fn default_transfer_z(entry: Option<&CatalogEntry>) -> f64 {
    let zs = crate::turntables_3d::extract_transfer_track_zs(entry);
    zs[zs.len() / 2] as f64
}

/// Returns active traversable track branches in world space.
/// For turntables and transfer tables, returns the single live animated bridge polyline.
pub(super) fn sample_active_piece_branches(
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    tt_angles: &HashMap<String, f64>,
    tr_offsets: &HashMap<String, f64>,
) -> Vec<Vec<Vec2>> {
    if let Some(e) = entry {
        let to_w = |lx: f64, ly: f64| -> Vec2 {
            let rp = railbench_core::geometry::rotate(Vec2 { x: lx, y: ly }, piece.transform.rotation);
            Vec2 {
                x: piece.transform.x + rp.x,
                y: piece.transform.y + rp.y,
            }
        };
        if e.kind == "turntable" {
            let outer_r = crate::turntables_3d::turntable_outer_radius(piece, entry);
            let ang = tt_angles.get(&piece.id).copied().unwrap_or(0.0).to_radians();
            let (ca, sa) = (ang.cos(), ang.sin());
            return vec![vec![
                to_w(-outer_r * ca, -outer_r * sa),
                to_w(outer_r * ca, outer_r * sa),
            ]];
        }
        if e.kind == "transfer-table" {
            let l = e.length.unwrap_or(360.0);
            let bz = tr_offsets
                .get(&piece.id)
                .copied()
                .unwrap_or_else(|| default_transfer_z(entry));
            return vec![vec![to_w(0.0, bz), to_w(l, bz)]];
        }
    }
    sample_track_branches_world(piece, entry)
}

/// Checks if a train can enter a turntable or transfer table via `entry_conn`.
/// Returns `Some((enter_at_start_of_bridge, bridge_len))` if aligned, or `None` if pit is open.
pub(super) fn check_special_entry(
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    entry_conn: usize,
    tt_angles: &HashMap<String, f64>,
    tr_offsets: &HashMap<String, f64>,
) -> Option<(bool, f64)> {
    let e = entry?;
    let c = piece.connectors.get(entry_conn)?;
    let local_c = railbench_core::geometry::rotate(
        Vec2 {
            x: c.x - piece.transform.x,
            y: c.y - piece.transform.y,
        },
        -piece.transform.rotation,
    );

    if e.kind == "turntable" {
        let outer_r = crate::turntables_3d::turntable_outer_radius(piece, entry);
        let bridge_ang = norm_deg_360(tt_angles.get(&piece.id).copied().unwrap_or(0.0));
        let conn_ang = norm_deg_360(local_c.y.atan2(local_c.x).to_degrees());
        if shortest_angle_diff_deg(conn_ang, bridge_ang + 180.0).abs() <= 5.5 {
            return Some((true, outer_r * 2.0));
        }
        if shortest_angle_diff_deg(conn_ang, bridge_ang).abs() <= 5.5 {
            return Some((false, outer_r * 2.0));
        }
        return None;
    }
    if e.kind == "transfer-table" {
        let l = e.length.unwrap_or(360.0);
        let bz = tr_offsets
            .get(&piece.id)
            .copied()
            .unwrap_or_else(|| default_transfer_z(entry));
        if (local_c.y - bz).abs() <= 6.0 {
            return Some((local_c.x < l * 0.5, l));
        }
        return None;
    }
    None
}

/// Finds the rim connector index on a turntable or transfer table matching the bridge exit endpoint.
pub(super) fn find_special_exit_connector(piece: &TrackPiece, exit_pt: Vec2) -> Option<usize> {
    let mut best: Option<(f64, usize)> = None;
    for (ci, c) in piece.connectors.iter().enumerate() {
        let d = (c.x - exit_pt.x).hypot(c.y - exit_pt.y);
        if d <= 8.0 && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, ci));
        }
    }
    best.map(|(_, ci)| ci)
}

fn layer_height_of(layout: &Layout, piece: &TrackPiece) -> f64 {
    let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
    layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == lid))
        .and_then(|l| l.height_mm)
        .unwrap_or(0.0)
}

pub(super) fn polyline_length(pts: &[Vec2]) -> f64 {
    pts.windows(2)
        .map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y))
        .sum()
}

fn sample_polyline_pose(pts: &[Vec2], dist: f64, e0: f64, e1: f64) -> (f64, f64, f64, f64, f64) {
    let total = polyline_length(pts).max(1e-6);
    if pts.is_empty() {
        return (0.0, 0.0, e0, 0.0, total);
    }
    if pts.len() == 1 {
        return (pts[0].x, pts[0].y, e0, 0.0, total);
    }
    let clamped = dist.clamp(0.0, total);
    let mut acc = 0.0;
    for w in pts.windows(2) {
        let dx = w[1].x - w[0].x;
        let dy = w[1].y - w[0].y;
        let seg = dx.hypot(dy);
        if acc + seg >= clamped || seg < 1e-9 {
            let t = if seg > 1e-9 {
                ((clamped - acc) / seg).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let wx = w[0].x + dx * t;
            let wy = w[0].y + dy * t;
            let wz = e0 + (e1 - e0) * (clamped / total);
            let ang = dy.atan2(dx).to_degrees();
            return (wx, wy, wz, ang, total);
        }
        acc += seg;
    }
    let last = pts[pts.len() - 1];
    let prev = pts[pts.len() - 2];
    (
        last.x,
        last.y,
        e1,
        (last.y - prev.y).atan2(last.x - prev.x).to_degrees(),
        total,
    )
}

pub(super) fn find_connected_next(
    layout: &Layout,
    catalogs: &[Catalog],
    piece: &TrackPiece,
    exit_conn: usize,
    domain: VehicleDomain,
) -> Option<(String, usize)> {
    let want_road = domain == VehicleDomain::Road;
    let c = piece.connectors.get(exit_conn)?;
    let cur_lh = layer_height_of(layout, piece);
    let cur_z = cur_lh
        + piece
            .elevation
            .map_or(0.0, |el| if exit_conn == 0 { el.start } else { el.end });

    let piece_allowed = |p: &TrackPiece| -> bool {
        if is_road_id(catalogs, &p.catalog_id) != want_road {
            return false;
        }
        if want_road
            && find_entry(catalogs, &p.catalog_id)
                .is_some_and(|e| matches!(e.kind.as_str(), "turntable" | "transfer-table"))
        {
            return false;
        }
        true
    };

    if let Some(ref link) = c.connected_to {
        if let Some(next_p) = layout.pieces.iter().find(|p| p.id == link.piece_id) {
            if piece_allowed(next_p) {
                return Some((link.piece_id.clone(), link.connector_index));
            }
        }
    }
    let mut best: Option<(f64, String, usize)> = None;
    for other in &layout.pieces {
        if other.id == piece.id || !piece_allowed(other) {
            continue;
        }
        let olh = layer_height_of(layout, other);
        for (ci, oc) in other.connectors.iter().enumerate() {
            let d = (oc.x - c.x).hypot(oc.y - c.y);
            if d >= 5.0 {
                continue;
            }
            let oz = olh
                + other
                    .elevation
                    .map_or(0.0, |el| if ci == 0 { el.start } else { el.end });
            if (oz - cur_z).abs() > 12.0 {
                continue;
            }
            let diff_a = (railbench_core::geometry::norm_angle_deg(oc.angle - c.angle) - 180.0).abs();
            if diff_a > 45.0 {
                continue;
            }
            if best.as_ref().is_none_or(|(bd, _, _)| d < *bd) {
                best = Some((d, other.id.clone(), ci));
            }
        }
    }
    best.map(|(_, pid, ci)| (pid, ci))
}

fn is_three_way_piece(piece: &TrackPiece, entry: Option<&CatalogEntry>) -> bool {
    if entry.is_some_and(|e| e.kind == "three-way") {
        return true;
    }
    if piece.connectors.len() != 4 {
        return false;
    }
    let a0 = piece.connectors[0].angle.to_radians();
    let (v0x, v0y) = (a0.cos(), a0.sin());
    piece.connectors[1..4]
        .iter()
        .filter(|ci| {
            let ai = ci.angle.to_radians();
            v0x * ai.cos() + v0y * ai.sin() < -0.15
        })
        .count()
        == 3
}

pub(super) fn branch_endpoint_connectors(
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    br_idx: usize,
) -> (usize, usize) {
    if entry.is_some_and(|e| e.kind == "double-slip") && piece.connectors.len() >= 4 {
        match br_idx {
            0 => (0, 1),
            1 => (2, 3),
            2 => (0, 3),
            3 => (2, 1),
            _ => (0, 1),
        }
    } else if piece.connectors.len() == 4 && !is_three_way_piece(piece, entry) {
        if br_idx == 0 { (0, 1) } else { (2, 3) }
    } else {
        (0, (br_idx + 1).min(piece.connectors.len().saturating_sub(1)))
    }
}

pub(super) fn resolve_piece_entry(
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    entry_conn: usize,
    turnout_state: usize,
    branch_count: usize,
    hist_branch: Option<usize>,
) -> (usize, bool, Option<usize>) {
    if entry.is_some_and(|e| e.kind == "double-slip")
        && piece.connectors.len() >= 4
        && branch_count >= 4
    {
        if let Some(hb) = hist_branch.filter(|&b| b < branch_count) {
            let (s, e) = branch_endpoint_connectors(piece, entry, hb);
            if s == entry_conn {
                return (hb, true, None);
            }
            if e == entry_conn {
                return (hb, false, None);
            }
        }
        return match (turnout_state % 2, entry_conn) {
            (0, 0) => (0, true, None),
            (0, 1) => (0, false, None),
            (0, 2) => (1, true, None),
            (0, 3) => (1, false, None),
            (1, 0) => (2, true, None),
            (1, 3) => (2, false, None),
            (1, 2) => (3, true, None),
            (1, 1) => (3, false, None),
            _ => (0, true, None),
        };
    }

    if piece.connectors.len() == 4 && !is_three_way_piece(piece, entry) {
        return match entry_conn {
            0 => (0, true, None),
            1 => (0, false, None),
            2 => (1, true, None),
            3 => (1, false, None),
            _ => (0, true, None),
        };
    }

    if entry_conn == 0 {
        let sw = hist_branch.unwrap_or(turnout_state);
        let br = sw.min(branch_count.saturating_sub(1));
        (br, true, None)
    } else {
        let br = (entry_conn - 1).min(branch_count.saturating_sub(1));
        let auto_st = if branch_count >= 2 { Some(br) } else { None };
        (br, false, auto_st)
    }
}

#[allow(clippy::too_many_arguments)]
fn transition_walk_to_next(
    layout: &Layout,
    catalogs: &[Catalog],
    turnout_states: &HashMap<String, usize>,
    tt_angles: &HashMap<String, f64>,
    tr_offsets: &HashMap<String, f64>,
    path_history: &[(String, usize)],
    next_pid: String,
    entry_conn: usize,
) -> Result<(String, usize, f64, bool), ()> {
    let next_piece = layout.pieces.iter().find(|p| p.id == next_pid);
    let next_entry = next_piece.and_then(|np| find_entry(catalogs, &np.catalog_id));
    let next_special = next_entry
        .is_some_and(|e| matches!(e.kind.as_str(), "turntable" | "transfer-table"));
    if next_special {
        if let Some(np) = next_piece {
            if let Some((at_start, blen)) =
                check_special_entry(np, next_entry, entry_conn, tt_angles, tr_offsets)
            {
                return Ok((
                    next_pid,
                    0,
                    if at_start { 0.0 } else { blen },
                    at_start,
                ));
            }
        }
        return Err(());
    }
    let next_branches = next_piece
        .map(|np| sample_active_piece_branches(np, next_entry, tt_angles, tr_offsets))
        .unwrap_or_default();
    let hist_br = path_history
        .iter()
        .rev()
        .find(|(pid, _)| *pid == next_pid)
        .map(|(_, b)| *b);
    let sw = turnout_states.get(&next_pid).copied().unwrap_or(0);
    let (br, at_start, _) = next_piece
        .map(|np| {
            resolve_piece_entry(
                np,
                next_entry,
                entry_conn,
                sw,
                next_branches.len(),
                hist_br,
            )
        })
        .unwrap_or((0, entry_conn == 0, None));
    let dist = if at_start {
        0.0
    } else {
        next_branches.get(br).map(|b| polyline_length(b)).unwrap_or(0.0)
    };
    Ok((next_pid, br, dist, at_start))
}

/// Walks a signed distance `signed_delta_mm` along the connected track/street graph.
#[allow(clippy::too_many_arguments)]
pub(super) fn walk_track_graph(
    layout: &Layout,
    catalogs: &[Catalog],
    turnout_states: &HashMap<String, usize>,
    tt_angles: &HashMap<String, f64>,
    tr_offsets: &HashMap<String, f64>,
    path_history: &[(String, usize)],
    start_piece_id: &str,
    start_branch_idx: usize,
    start_dist_mm: f64,
    signed_delta_mm: f64,
    domain: VehicleDomain,
) -> (f64, f64, f64, f64) {
    let mut cur_pid = start_piece_id.to_string();
    let mut cur_br = start_branch_idx;
    let mut cur_dist = start_dist_mm;
    let mut dir_pos = signed_delta_mm >= 0.0;
    let mut remain = signed_delta_mm.abs();

    for _hop in 0..16 {
        let Some(piece) = layout.pieces.iter().find(|p| p.id == cur_pid) else {
            break;
        };
        if is_road_id(catalogs, &piece.catalog_id) != (domain == VehicleDomain::Road) {
            break;
        }
        let entry = find_entry(catalogs, &piece.catalog_id);
        let is_special = entry
            .is_some_and(|e| matches!(e.kind.as_str(), "turntable" | "transfer-table"));
        let branches = sample_active_piece_branches(piece, entry, tt_angles, tr_offsets);
        if branches.is_empty() {
            break;
        }
        let br_idx = cur_br.min(branches.len() - 1);
        let br = &branches[br_idx];
        let seg_len = polyline_length(br).max(1e-3);
        let lh = layer_height_of(layout, piece);
        let (e0, e1) = piece.elevation.map_or((lh, lh), |el| (lh + el.start, lh + el.end));

        if dir_pos {
            if cur_dist + remain <= seg_len {
                let (wx, wy, wz, ang, _) = sample_polyline_pose(br, cur_dist + remain, e0, e1);
                return (wx, wy, wz, ang);
            }
            remain -= (seg_len - cur_dist).max(0.0);
            let exit_conn = if is_special {
                br.last().copied().and_then(|pt| find_special_exit_connector(piece, pt))
            } else {
                Some(branch_endpoint_connectors(piece, entry, br_idx).1)
            };
            if let Some((next_pid, entry_conn)) = exit_conn
                .and_then(|ec| find_connected_next(layout, catalogs, piece, ec, domain))
            {
                if let Ok((np, nb, nd, nf)) = transition_walk_to_next(
                    layout,
                    catalogs,
                    turnout_states,
                    tt_angles,
                    tr_offsets,
                    path_history,
                    next_pid,
                    entry_conn,
                ) {
                    cur_pid = np;
                    cur_br = nb;
                    cur_dist = nd;
                    dir_pos = nf;
                    continue;
                }
            }
            let (wx, wy, wz, ang, _) = sample_polyline_pose(br, seg_len, e0, e1);
            let rad = ang.to_radians();
            let slope = (e1 - e0) / seg_len;
            return (wx + rad.cos() * remain, wy + rad.sin() * remain, wz + slope * remain, ang);
        } else {
            if cur_dist - remain >= 0.0 {
                let (wx, wy, wz, ang, _) = sample_polyline_pose(br, cur_dist - remain, e0, e1);
                return (wx, wy, wz, ang + 180.0);
            }
            remain -= cur_dist.max(0.0);
            let exit_conn = if is_special {
                br.first().copied().and_then(|pt| find_special_exit_connector(piece, pt))
            } else {
                Some(branch_endpoint_connectors(piece, entry, br_idx).0)
            };
            if let Some((next_pid, entry_conn)) = exit_conn
                .and_then(|ec| find_connected_next(layout, catalogs, piece, ec, domain))
            {
                if let Ok((np, nb, nd, nf)) = transition_walk_to_next(
                    layout,
                    catalogs,
                    turnout_states,
                    tt_angles,
                    tr_offsets,
                    path_history,
                    next_pid,
                    entry_conn,
                ) {
                    cur_pid = np;
                    cur_br = nb;
                    cur_dist = nd;
                    dir_pos = nf;
                    continue;
                }
            }
            let (wx, wy, wz, ang, _) = sample_polyline_pose(br, 0.0, e0, e1);
            let rad = (ang + 180.0).to_radians();
            let slope = (e0 - e1) / seg_len;
            return (
                wx + rad.cos() * remain,
                wy + rad.sin() * remain,
                wz + slope * remain,
                ang + 180.0,
            );
        }
    }
    (0.0, 0.0, 0.0, 0.0)
}

/// Walks `signed_delta_mm` along the connected track graph and returns `(piece_id, branch_idx, dist_mm, travel_dir_pos)`.
#[allow(clippy::too_many_arguments)]
pub(super) fn walk_track_graph_location(
    layout: &Layout,
    catalogs: &[Catalog],
    turnout_states: &HashMap<String, usize>,
    tt_angles: &HashMap<String, f64>,
    tr_offsets: &HashMap<String, f64>,
    path_history: &[(String, usize)],
    start_piece_id: &str,
    start_branch_idx: usize,
    start_dist_mm: f64,
    signed_delta_mm: f64,
    domain: VehicleDomain,
) -> (String, usize, f64, bool) {
    let mut cur_pid = start_piece_id.to_string();
    let mut cur_br = start_branch_idx;
    let mut cur_dist = start_dist_mm;
    let mut dir_pos = signed_delta_mm >= 0.0;
    let mut remain = signed_delta_mm.abs();

    for _hop in 0..16 {
        let Some(piece) = layout.pieces.iter().find(|p| p.id == cur_pid) else {
            break;
        };
        if is_road_id(catalogs, &piece.catalog_id) != (domain == VehicleDomain::Road) {
            break;
        }
        let entry = find_entry(catalogs, &piece.catalog_id);
        let is_special = entry
            .is_some_and(|e| matches!(e.kind.as_str(), "turntable" | "transfer-table"));
        let branches = sample_active_piece_branches(piece, entry, tt_angles, tr_offsets);
        if branches.is_empty() {
            break;
        }
        let br_idx = cur_br.min(branches.len() - 1);
        let br = &branches[br_idx];
        let seg_len = polyline_length(br).max(1e-3);

        if dir_pos {
            if cur_dist + remain <= seg_len {
                return (cur_pid, br_idx, cur_dist + remain, true);
            }
            remain -= (seg_len - cur_dist).max(0.0);
            let exit_conn = if is_special {
                br.last().copied().and_then(|pt| find_special_exit_connector(piece, pt))
            } else {
                Some(branch_endpoint_connectors(piece, entry, br_idx).1)
            };
            if let Some((next_pid, entry_conn)) = exit_conn
                .and_then(|ec| find_connected_next(layout, catalogs, piece, ec, domain))
            {
                if let Ok((np, nb, nd, nf)) = transition_walk_to_next(
                    layout,
                    catalogs,
                    turnout_states,
                    tt_angles,
                    tr_offsets,
                    path_history,
                    next_pid,
                    entry_conn,
                ) {
                    cur_pid = np;
                    cur_br = nb;
                    cur_dist = nd;
                    dir_pos = nf;
                    continue;
                }
            }
            return (cur_pid, br_idx, seg_len, true);
        } else {
            if cur_dist - remain >= 0.0 {
                return (cur_pid, br_idx, cur_dist - remain, false);
            }
            remain -= cur_dist.max(0.0);
            let exit_conn = if is_special {
                br.first().copied().and_then(|pt| find_special_exit_connector(piece, pt))
            } else {
                Some(branch_endpoint_connectors(piece, entry, br_idx).0)
            };
            if let Some((next_pid, entry_conn)) = exit_conn
                .and_then(|ec| find_connected_next(layout, catalogs, piece, ec, domain))
            {
                if let Ok((np, nb, nd, nf)) = transition_walk_to_next(
                    layout,
                    catalogs,
                    turnout_states,
                    tt_angles,
                    tr_offsets,
                    path_history,
                    next_pid,
                    entry_conn,
                ) {
                    cur_pid = np;
                    cur_br = nb;
                    cur_dist = nd;
                    dir_pos = nf;
                    continue;
                }
            }
            return (cur_pid, br_idx, 0.0, false);
        }
    }
    (cur_pid, cur_br, cur_dist, dir_pos)
}

#[allow(clippy::too_many_arguments)]
fn sample_bogie_car_pose(
    layout: &Layout,
    catalogs: &[Catalog],
    turnout_states: &HashMap<String, usize>,
    tt_angles: &HashMap<String, f64>,
    tr_offsets: &HashMap<String, f64>,
    path_history: &[(String, usize)],
    piece_id: &str,
    branch_idx: usize,
    dist_mm: f64,
    orient_sign: f64,
    center_offset_along_orient: f64,
    bogie_half_span: f64,
    domain: VehicleDomain,
) -> (f64, f64, f64, f64, f64) {
    let f_delta = orient_sign * (center_offset_along_orient + bogie_half_span);
    let r_delta = orient_sign * (center_offset_along_orient - bogie_half_span);
    let (fx, fy, fz, _) = walk_track_graph(
        layout,
        catalogs,
        turnout_states,
        tt_angles,
        tr_offsets,
        path_history,
        piece_id,
        branch_idx,
        dist_mm,
        f_delta,
        domain,
    );
    let (rx, ry, rz, _) = walk_track_graph(
        layout,
        catalogs,
        turnout_states,
        tt_angles,
        tr_offsets,
        path_history,
        piece_id,
        branch_idx,
        dist_mm,
        r_delta,
        domain,
    );
    let dx = fx - rx;
    let dy = fy - ry;
    let dz = fz - rz;
    let horiz = dx.hypot(dy).max(1e-6);
    ((fx + rx) * 0.5, (fy + ry) * 0.5, (fz + rz) * 0.5, dy.atan2(dx).to_degrees(), dz.atan2(horiz))
}

/// Returns `(scale_factor_from_h0, gauge_mm)` tailored to the exact track catalog under `piece_id`
/// (falling back to `layout.scale` for roads or untagged tracks) so train wheelsets always match the rails.
pub(super) fn track_scale_and_gauge(
    layout: &Layout,
    catalogs: &[Catalog],
    piece_id: &str,
    domain: VehicleDomain,
) -> (f64, f64) {
    if domain == VehicleDomain::Rail {
        if let Some(piece) = layout.pieces.iter().find(|p| p.id == piece_id) {
            if let Some(g) = find_entry(catalogs, &piece.catalog_id).and_then(|e| e.gauge) {
                if g > 2.0 {
                    return ((g / 16.5).clamp(0.35, 3.0), g);
                }
            }
        }
    }
    let sf = scale_factor_from_h0(&layout.scale).max(0.1);
    (sf, 16.5 * sf)
}

pub(super) fn recompute_vehicle_poses(
    v: &mut VehicleInstance,
    layout: &Layout,
    catalogs: &[Catalog],
    turnout_states: &HashMap<String, usize>,
    tt_angles: &HashMap<String, f64>,
    tr_offsets: &HashMap<String, f64>,
) {
    let (sf, gauge_mm) = track_scale_and_gauge(layout, catalogs, &v.piece_id, v.domain);
    let def = v.kind.def();
    let l = def.length_h0_mm * sf;
    let w = def.width_h0_mm * sf;
    let h = def.height_h0_mm * sf;
    let bogie = (def.bogie_half_span_h0_mm * sf).max(4.0);
    let orient_sign = if v.forward { 1.0 } else { -1.0 };

    let (cx, cy, cz, hdg, pitch) = sample_bogie_car_pose(
        layout,
        catalogs,
        turnout_states,
        tt_angles,
        tr_offsets,
        &v.path_history,
        &v.piece_id,
        v.branch_idx,
        v.dist_mm,
        orient_sign,
        0.0,
        bogie,
        v.domain,
    );
    v.world_x = cx;
    v.world_y = cy;
    v.world_z = cz;
    v.heading_deg = hdg;
    v.pitch_rad = pitch;
    let travel_sign = if v.velocity_mm_s < -0.5 || (v.velocity_mm_s.abs() <= 0.5 && v.reverse) {
        -1.0
    } else {
        1.0
    };
    v.grade_pct = (pitch.tan() * 100.0 * travel_sign).clamp(-25.0, 25.0);
    let driving_backward = v.velocity_mm_s < -1.0 || (v.velocity_mm_s.abs() <= 1.0 && v.reverse);
    v.mallet_articulation_rad = if v.kind == super::VehicleKind::Br96Mallet {
        let (_, _, _, ahead_ang) = walk_track_graph(
            layout,
            catalogs,
            turnout_states,
            tt_angles,
            tr_offsets,
            &v.path_history,
            &v.piece_id,
            v.branch_idx,
            v.dist_mm,
            orient_sign * 50.0 * sf,
            v.domain,
        );
        shortest_angle_diff_deg(hdg, ahead_ang)
            .to_radians()
            .clamp(-0.32, 0.32) as f32
    } else {
        0.0
    };
    let speed_norm = (v.current_speed_mm_s / (def.max_speed_mm_s * sf).max(1.0)).clamp(0.0, 1.0) as f32;

    let mut poses = Vec::with_capacity(1 + v.coupled_cars.len());
    poses.push(VehicleRenderPose {
        vehicle_id: v.id.clone(),
        label: v.name.clone(),
        kind: v.kind,
        x: cx,
        y: cy,
        z: cz,
        heading_deg: hdg,
        pitch_rad: pitch,
        length_mm: l,
        width_mm: w,
        height_mm: h,
        gauge_mm,
        livery_rgb: v.livery_rgb,
        headlights: v.headlights,
        driving_backward,
        throttle_pct: v.throttle * 100.0,
        wheel_angle: v.wheel_angle,
        pantograph_up: v.pantograph_up,
        steam_on: v.steam_on,
        steam_time: v.steam_time,
        speed_norm,
        mallet_articulation_rad: v.mallet_articulation_rad,
    });

    let coupler_gap = (3.5 + 1.4 * v.coupler_strain as f64).clamp(1.8, 6.5) * sf;
    let mut trail_offset = l * 0.5 + coupler_gap;
    for &wk in &v.coupled_cars {
        let wdef = wk.def();
        let wl = wdef.length_h0_mm * sf;
        let ww = wdef.width_h0_mm * sf;
        let wh = wdef.height_h0_mm * sf;
        let wbogie = (wdef.bogie_half_span_h0_mm * sf).max(4.0);
        let center_trail = trail_offset + wl * 0.5;
        let (wx, wy, wz, whdg, wpitch) = sample_bogie_car_pose(
            layout,
            catalogs,
            turnout_states,
            tt_angles,
            tr_offsets,
            &v.path_history,
            &v.piece_id,
            v.branch_idx,
            v.dist_mm,
            orient_sign,
            -center_trail,
            wbogie,
            v.domain,
        );
        poses.push(VehicleRenderPose {
            vehicle_id: v.id.clone(),
            label: String::new(),
            kind: wk,
            x: wx,
            y: wy,
            z: wz,
            heading_deg: whdg,
            pitch_rad: wpitch,
            length_mm: wl,
            width_mm: ww,
            height_mm: wh,
            gauge_mm,
            livery_rgb: wdef.default_rgb,
            headlights: v.headlights,
            driving_backward,
            throttle_pct: 0.0,
            wheel_angle: 0.0,
            pantograph_up: false,
            steam_on: false,
            steam_time: 0.0,
            speed_norm: 0.0,
            mallet_articulation_rad: 0.0,
        });
        trail_offset += wl + coupler_gap;
    }
    v.cached_poses = poses;
}

/// Cached variant of `walk_track_graph` that reuses sampled polylines and O(1) piece lookups.
#[allow(clippy::too_many_arguments)]
pub(super) fn walk_track_graph_cached(
    cache: &mut TrackWalkCache<'_>,
    turnout_states: &HashMap<String, usize>,
    path_history: &[(String, usize)],
    start_piece_id: &str,
    start_branch_idx: usize,
    start_dist_mm: f64,
    signed_delta_mm: f64,
    domain: VehicleDomain,
) -> (f64, f64, f64, f64) {
    let Some(mut cur_idx) = cache.piece_idx(start_piece_id) else {
        return (0.0, 0.0, 0.0, 0.0);
    };
    let mut cur_br = start_branch_idx;
    let mut cur_dist = start_dist_mm;
    let mut dir_pos = signed_delta_mm >= 0.0;
    let mut remain = signed_delta_mm.abs();

    for _hop in 0..16 {
        let piece = cache.piece(cur_idx);
        let (is_special, entry, br_count, br_idx, seg_len, e0, e1) = {
            let cached = cache.get_or_sample(cur_idx);
            if cached.is_road != (domain == VehicleDomain::Road) || cached.branches.is_empty() {
                break;
            }
            let bi = cur_br.min(cached.branches.len() - 1);
            let sl = cached.lengths[bi].max(1e-3);
            let lh = cached.layer_h;
            let (e0, e1) = piece
                .elevation
                .map_or((lh, lh), |el| (lh + el.start, lh + el.end));
            (cached.is_special, cached.entry, cached.branches.len(), bi, sl, e0, e1)
        };
        let _ = br_count;

        if dir_pos {
            if cur_dist + remain <= seg_len {
                let br = &cache.get_or_sample(cur_idx).branches[br_idx];
                let (wx, wy, wz, ang, _) = sample_polyline_pose(br, cur_dist + remain, e0, e1);
                return (wx, wy, wz, ang);
            }
            remain -= (seg_len - cur_dist).max(0.0);
            let exit_conn = if is_special {
                let last_pt = cache.get_or_sample(cur_idx).branches[br_idx].last().copied();
                last_pt.and_then(|pt| find_special_exit_connector(piece, pt))
            } else {
                Some(branch_endpoint_connectors(piece, entry, br_idx).1)
            };
            if let Some((next_idx, entry_conn)) =
                exit_conn.and_then(|ec| cache.find_connected_next(cur_idx, ec, domain))
            {
                if let Ok((ni, nb, nd, nf, _)) =
                    cache.resolve_transition(turnout_states, path_history, next_idx, entry_conn, true)
                {
                    cur_idx = ni;
                    cur_br = nb;
                    cur_dist = nd;
                    dir_pos = nf;
                    continue;
                }
            }
            let br = &cache.get_or_sample(cur_idx).branches[br_idx];
            let (wx, wy, wz, ang, _) = sample_polyline_pose(br, seg_len, e0, e1);
            let rad = ang.to_radians();
            let slope = (e1 - e0) / seg_len;
            return (wx + rad.cos() * remain, wy + rad.sin() * remain, wz + slope * remain, ang);
        } else {
            if cur_dist - remain >= 0.0 {
                let br = &cache.get_or_sample(cur_idx).branches[br_idx];
                let (wx, wy, wz, ang, _) = sample_polyline_pose(br, cur_dist - remain, e0, e1);
                return (wx, wy, wz, ang + 180.0);
            }
            remain -= cur_dist.max(0.0);
            let exit_conn = if is_special {
                let first_pt = cache.get_or_sample(cur_idx).branches[br_idx].first().copied();
                first_pt.and_then(|pt| find_special_exit_connector(piece, pt))
            } else {
                Some(branch_endpoint_connectors(piece, entry, br_idx).0)
            };
            if let Some((next_idx, entry_conn)) =
                exit_conn.and_then(|ec| cache.find_connected_next(cur_idx, ec, domain))
            {
                if let Ok((ni, nb, nd, nf, _)) =
                    cache.resolve_transition(turnout_states, path_history, next_idx, entry_conn, true)
                {
                    cur_idx = ni;
                    cur_br = nb;
                    cur_dist = nd;
                    dir_pos = nf;
                    continue;
                }
            }
            let br = &cache.get_or_sample(cur_idx).branches[br_idx];
            let (wx, wy, wz, ang, _) = sample_polyline_pose(br, 0.0, e0, e1);
            let rad = (ang + 180.0).to_radians();
            let slope = (e0 - e1) / seg_len;
            return (
                wx + rad.cos() * remain,
                wy + rad.sin() * remain,
                wz + slope * remain,
                ang + 180.0,
            );
        }
    }
    (0.0, 0.0, 0.0, 0.0)
}

#[allow(clippy::too_many_arguments)]
fn sample_bogie_car_pose_cached(
    cache: &mut TrackWalkCache<'_>,
    turnout_states: &HashMap<String, usize>,
    path_history: &[(String, usize)],
    piece_id: &str,
    branch_idx: usize,
    dist_mm: f64,
    orient_sign: f64,
    center_offset_along_orient: f64,
    bogie_half_span: f64,
    domain: VehicleDomain,
) -> (f64, f64, f64, f64, f64) {
    let f_delta = orient_sign * (center_offset_along_orient + bogie_half_span);
    let r_delta = orient_sign * (center_offset_along_orient - bogie_half_span);
    let (fx, fy, fz, _) = walk_track_graph_cached(
        cache,
        turnout_states,
        path_history,
        piece_id,
        branch_idx,
        dist_mm,
        f_delta,
        domain,
    );
    let (rx, ry, rz, _) = walk_track_graph_cached(
        cache,
        turnout_states,
        path_history,
        piece_id,
        branch_idx,
        dist_mm,
        r_delta,
        domain,
    );
    let dx = fx - rx;
    let dy = fy - ry;
    let dz = fz - rz;
    let horiz = dx.hypot(dy).max(1e-6);
    (
        (fx + rx) * 0.5,
        (fy + ry) * 0.5,
        (fz + rz) * 0.5,
        dy.atan2(dx).to_degrees(),
        dz.atan2(horiz),
    )
}

/// Cached variant of `recompute_vehicle_poses` reusing `TrackWalkCache` across the entire consist.
pub(super) fn recompute_vehicle_poses_cached(
    v: &mut VehicleInstance,
    cache: &mut TrackWalkCache<'_>,
    turnout_states: &HashMap<String, usize>,
) {
    let (sf, gauge_mm) = cache.track_scale_and_gauge(&v.piece_id, v.domain);
    let def = v.kind.def();
    let l = def.length_h0_mm * sf;
    let w = def.width_h0_mm * sf;
    let h = def.height_h0_mm * sf;
    let bogie = (def.bogie_half_span_h0_mm * sf).max(4.0);
    let orient_sign = if v.forward { 1.0 } else { -1.0 };

    let (cx, cy, cz, hdg, pitch) = sample_bogie_car_pose_cached(
        cache,
        turnout_states,
        &v.path_history,
        &v.piece_id,
        v.branch_idx,
        v.dist_mm,
        orient_sign,
        0.0,
        bogie,
        v.domain,
    );
    v.world_x = cx;
    v.world_y = cy;
    v.world_z = cz;
    v.heading_deg = hdg;
    v.pitch_rad = pitch;
    let travel_sign = if v.velocity_mm_s < -0.5 || (v.velocity_mm_s.abs() <= 0.5 && v.reverse) {
        -1.0
    } else {
        1.0
    };
    v.grade_pct = (pitch.tan() * 100.0 * travel_sign).clamp(-25.0, 25.0);
    let driving_backward = v.velocity_mm_s < -1.0 || (v.velocity_mm_s.abs() <= 1.0 && v.reverse);
    v.mallet_articulation_rad = if v.kind == super::VehicleKind::Br96Mallet {
        let (_, _, _, ahead_ang) = walk_track_graph_cached(
            cache,
            turnout_states,
            &v.path_history,
            &v.piece_id,
            v.branch_idx,
            v.dist_mm,
            orient_sign * 50.0 * sf,
            v.domain,
        );
        shortest_angle_diff_deg(hdg, ahead_ang)
            .to_radians()
            .clamp(-0.32, 0.32) as f32
    } else {
        0.0
    };
    let speed_norm =
        (v.current_speed_mm_s / (def.max_speed_mm_s * sf).max(1.0)).clamp(0.0, 1.0) as f32;

    let mut poses = Vec::with_capacity(1 + v.coupled_cars.len());
    poses.push(VehicleRenderPose {
        vehicle_id: v.id.clone(),
        label: v.name.clone(),
        kind: v.kind,
        x: cx,
        y: cy,
        z: cz,
        heading_deg: hdg,
        pitch_rad: pitch,
        length_mm: l,
        width_mm: w,
        height_mm: h,
        gauge_mm,
        livery_rgb: v.livery_rgb,
        headlights: v.headlights,
        driving_backward,
        throttle_pct: v.throttle * 100.0,
        wheel_angle: v.wheel_angle,
        pantograph_up: v.pantograph_up,
        steam_on: v.steam_on,
        steam_time: v.steam_time,
        speed_norm,
        mallet_articulation_rad: v.mallet_articulation_rad,
    });

    let coupler_gap = (3.5 + 1.4 * v.coupler_strain as f64).clamp(1.8, 6.5) * sf;
    let mut trail_offset = l * 0.5 + coupler_gap;
    for &wk in &v.coupled_cars {
        let wdef = wk.def();
        let wl = wdef.length_h0_mm * sf;
        let ww = wdef.width_h0_mm * sf;
        let wh = wdef.height_h0_mm * sf;
        let wbogie = (wdef.bogie_half_span_h0_mm * sf).max(4.0);
        let center_trail = trail_offset + wl * 0.5;
        let (wx, wy, wz, whdg, wpitch) = sample_bogie_car_pose_cached(
            cache,
            turnout_states,
            &v.path_history,
            &v.piece_id,
            v.branch_idx,
            v.dist_mm,
            orient_sign,
            -center_trail,
            wbogie,
            v.domain,
        );
        poses.push(VehicleRenderPose {
            vehicle_id: v.id.clone(),
            label: String::new(),
            kind: wk,
            x: wx,
            y: wy,
            z: wz,
            heading_deg: whdg,
            pitch_rad: wpitch,
            length_mm: wl,
            width_mm: ww,
            height_mm: wh,
            gauge_mm,
            livery_rgb: wdef.default_rgb,
            headlights: v.headlights,
            driving_backward,
            throttle_pct: 0.0,
            wheel_angle: 0.0,
            pantograph_up: false,
            steam_on: false,
            steam_time: 0.0,
            speed_norm: 0.0,
            mallet_articulation_rad: 0.0,
        });
        trail_offset += wl + coupler_gap;
    }
    v.cached_poses = poses;
}
