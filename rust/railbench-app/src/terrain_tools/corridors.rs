use super::*;

pub fn create_tunnel_profile(
    is_road: bool,
    has_catenary: bool,
    gauge: f64,
    num_tracks: usize,
    track_spacing: f64,
) -> TunnelProfile {
    if is_road {
        return TunnelProfile {
            is_road: true,
            has_catenary: false,
            num_tracks: 1,
            inner_width: 86.0,
            wall_h: 54.0,
            arch_rise: 24.0,
            pier: 15.0,
            total_h: 78.0,
        };
    }

    let scale = (gauge / 16.5).clamp(0.4, 3.0) as f32;
    let base_single_w = (if has_catenary { 74.0 } else { 60.0 }) * scale;
    let base_wall_h = (if has_catenary { 76.0 } else { 56.0 }) * scale;
    let base_arch_rise = (if has_catenary { 36.0 } else { 28.0 }) * scale;
    let pier = (16.0 * scale).round();

    if num_tracks <= 1 {
        let w = base_single_w.round();
        let wh = base_wall_h.round();
        let ar = base_arch_rise.round();
        return TunnelProfile {
            is_road: false,
            has_catenary,
            num_tracks: 1,
            inner_width: w,
            wall_h: wh,
            arch_rise: ar,
            pier,
            total_h: wh + ar,
        };
    }

    let total_spread = (track_spacing as f32).max(((num_tracks - 1) as f32) * 58.0 * scale);
    let multi_w = (total_spread + base_single_w).round();
    let multi_wh = ((if has_catenary { 78.0 } else { 58.0 }) * scale).round();
    let multi_ar = ((if has_catenary { 42.0 } else { 32.0 }) * scale).round();
    TunnelProfile {
        is_road: false,
        has_catenary,
        num_tracks,
        inner_width: multi_w,
        wall_h: multi_wh,
        arch_rise: multi_ar,
        pier: (pier * 1.15).round(),
        total_h: multi_wh + multi_ar,
    }
}

/// Cross-section points from bottom-left over the circular arch to bottom-right.
pub fn tunnel_profile_points(prof: &TunnelProfile, inset: f32, arc_steps: usize) -> Vec<[f32; 2]> {
    let steps = arc_steps.max(4);
    let c = (prof.inner_width * 0.5 - inset).max(6.0);
    let rise = (prof.arch_rise - inset).max(2.0);
    let wall_h = prof.wall_h.max(4.0);

    let r_circle = (c * c + rise * rise) / (2.0 * rise);
    let cy = wall_h + rise - r_circle;
    let th_l = (wall_h - cy).atan2(-c);
    let th_r = (wall_h - cy).atan2(c);

    let mut pts = Vec::with_capacity(steps + 3);
    pts.push([-c, 0.0]);
    pts.push([-c, wall_h]);
    for i in 1..steps {
        let th = th_l + (th_r - th_l) * (i as f32 / steps as f32);
        pts.push([r_circle * th.cos(), cy + r_circle * th.sin()]);
    }
    pts.push([c, wall_h]);
    pts.push([c, 0.0]);
    pts
}

#[derive(Debug, Clone)]
pub struct CorridorChain {
    pub pts_xze: Vec<[f32; 3]>, // [world_x, world_z, world_elev]
    pub half_w: f32,
    pub is_road: bool,
    pub has_catenary: bool,
    pub gauge: f64,
    pub is_turntable: bool,
    pub start_connected: bool,
    pub end_connected: bool,
}

pub(super) fn piece_world_elev(piece: &TrackPiece, layer_h: f64, conn_idx: usize) -> f64 {
    let ev = piece.elevation.unwrap_or_default();
    if conn_idx == 0 {
        layer_h + ev.start
    } else {
        layer_h + ev.end
    }
}

pub(super) fn sample_two_conn_segment_xze(
    piece: &TrackPiece,
    entry_c: usize,
    exit_c: usize,
    layer_h: f64,
) -> Vec<[f32; 3]> {
    let e_in = piece_world_elev(piece, layer_h, entry_c);
    let e_out = piece_world_elev(piece, layer_h, exit_c);

    if let (Some(offsets), Some(chord)) = (&piece.custom_offsets, piece.custom_chord_dist) {
        if offsets.len() >= 3 {
            let steps = 20usize;
            let flat = sample_flex_spline_points(chord, offsets[0], offsets[1], offsets[2], steps);
            let mut raw = Vec::with_capacity(steps + 1);
            for k in 0..=steps {
                let u = k as f64 / steps as f64;
                let rp = rotate(Vec2 { x: flat[k * 5], y: flat[k * 5 + 1] }, piece.transform.rotation);
                let e_pt = if entry_c == 0 {
                    e_in + (e_out - e_in) * u
                } else {
                    e_out + (e_in - e_out) * u
                };
                raw.push([
                    (piece.transform.x + rp.x) as f32,
                    (piece.transform.y + rp.y) as f32,
                    e_pt as f32,
                ]);
            }
            if entry_c == 1 {
                raw.reverse();
            }
            return raw;
        }
    }

    let c = &piece.connectors;
    if entry_c >= c.len() || exit_c >= c.len() {
        return Vec::new();
    }
    let chord = (c[exit_c].x - c[entry_c].x).hypot(c[exit_c].y - c[entry_c].y);
    let steps = ((chord / 14.0).ceil() as usize).clamp(4, 48);
    let pts2d = sample_arc_between_connectors(
        c[entry_c].x,
        c[entry_c].y,
        c[entry_c].angle,
        c[exit_c].x,
        c[exit_c].y,
        c[exit_c].angle,
        steps,
    );
    let n = pts2d.len().saturating_sub(1).max(1);
    pts2d
        .into_iter()
        .enumerate()
        .map(|(k, p)| {
            let u = k as f64 / n as f64;
            [p.x as f32, p.y as f32, (e_in + (e_out - e_in) * u) as f32]
        })
        .collect()
}

pub(super) fn push_dense_subdivided(out: &mut Vec<[f32; 3]>, raw: &[[f32; 3]]) {
    for (si, w) in raw.windows(2).enumerate() {
        let a = w[0];
        let b = w[1];
        let seg_len = (b[0] - a[0]).hypot(b[1] - a[1]);
        let sub = ((seg_len / 14.0).ceil() as usize).max(1);
        let start_k = if si == 0 && out.is_empty() { 0 } else { 1 };
        for k in start_k..=sub {
            let u = k as f32 / sub as f32;
            let cand = [
                a[0] + (b[0] - a[0]) * u,
                a[1] + (b[1] - a[1]) * u,
                a[2] + (b[2] - a[2]) * u,
            ];
            if let Some(last) = out.last() {
                if (cand[0] - last[0]).hypot(cand[1] - last[1]) < 0.8 {
                    continue;
                }
            }
            out.push(cand);
        }
    }
}

/// Builds continuously chained track & road corridors matching TS `groupConnectedPieces`
/// so multi-segment tracks passing through mountains produce seamless tunnels without fake interior portals.
pub fn build_connected_corridor_chains(
    layout: &Layout,
    catalogs: &[Catalog],
    layer_info: &HashMap<&str, (f64, bool)>,
) -> Vec<CorridorChain> {
    let mut visible: Vec<&TrackPiece> = Vec::new();
    let mut id_to_vidx: HashMap<&str, usize> = HashMap::new();

    for p in &layout.pieces {
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let vis = layer_info.get(lid).map(|x| x.1).unwrap_or(true);
        if vis {
            let vi = visible.len();
            id_to_vidx.insert(p.id.as_str(), vi);
            visible.push(p);
        }
    }

    let is_two_conn_chainable = |p: &TrackPiece| p.connectors.len() == 2;

    let mut out = Vec::new();
    let mut visited_two = vec![false; visible.len()];

    // Pass 1: Chain all 2-connector pieces separately by road vs rail
    for target_road in [false, true] {
        // First start from chain endpoints (connectors NOT leading to another 2-conn piece of same type)
        for pass_loops in [false, true] {
            for start_i in 0..visible.len() {
                if visited_two[start_i] {
                    continue;
                }
                let p_start = visible[start_i];
                if !is_two_conn_chainable(p_start)
                    || is_road_id(catalogs, &p_start.catalog_id) != target_road
                {
                    continue;
                }

                let neighbor_is_chainable = |p: &TrackPiece, c_idx: usize| -> bool {
                    let Some(ref link) = p.connectors[c_idx].connected_to else {
                        return false;
                    };
                    let Some(&ni) = id_to_vidx.get(link.piece_id.as_str()) else {
                        return false;
                    };
                    let np = visible[ni];
                    is_two_conn_chainable(np)
                        && is_road_id(catalogs, &np.catalog_id) == target_road
                        && link.connector_index < 2
                };

                let entry_c = if !pass_loops {
                    if !neighbor_is_chainable(p_start, 0) {
                        0usize
                    } else if !neighbor_is_chainable(p_start, 1) {
                        1usize
                    } else {
                        continue;
                    }
                } else {
                    0usize
                };

                let mut pts_xze = Vec::new();
                let mut cur_idx = start_i;
                let mut cur_entry = entry_c;
                let mut has_catenary = false;
                let mut gauge = 16.5_f64;
                let mut half_w_max = 20.0_f32;
                let start_connected = p_start.connectors[entry_c].connected_to.is_some();
                let mut end_connected = false;

                loop {
                    if visited_two[cur_idx] {
                        break;
                    }
                    visited_two[cur_idx] = true;
                    let cp = visible[cur_idx];
                    let cur_exit = 1 - cur_entry;
                    if cp.has_catenary == Some(true) {
                        has_catenary = true;
                    }
                    let entry_meta = find_entry(catalogs, &cp.catalog_id);
                    if let Some(g) = entry_meta.and_then(|e| e.gauge) {
                        gauge = g;
                    }
                    let hw = if target_road {
                        entry_meta.and_then(|e| e.road_width).unwrap_or(60.0) as f32 * 0.5
                    } else {
                        ((gauge / 16.5) * 20.0) as f32
                    };
                    half_w_max = half_w_max.max(hw);

                    let lid = cp.layer_id.as_deref().unwrap_or("layer-1");
                    let lh = layer_info.get(lid).map(|x| x.0).unwrap_or(0.0);
                    let seg = sample_two_conn_segment_xze(cp, cur_entry, cur_exit, lh);
                    push_dense_subdivided(&mut pts_xze, &seg);

                    end_connected = cp.connectors[cur_exit].connected_to.is_some();
                    let Some(ref link) = cp.connectors[cur_exit].connected_to else {
                        break;
                    };
                    let Some(&next_idx) = id_to_vidx.get(link.piece_id.as_str()) else {
                        break;
                    };
                    let np = visible[next_idx];
                    if visited_two[next_idx]
                        || !is_two_conn_chainable(np)
                        || is_road_id(catalogs, &np.catalog_id) != target_road
                        || link.connector_index >= 2
                    {
                        break;
                    }
                    cur_idx = next_idx;
                    cur_entry = link.connector_index;
                }

                if pts_xze.len() >= 2 {
                    out.push(CorridorChain {
                        pts_xze,
                        half_w: half_w_max,
                        is_road: target_road,
                        has_catenary,
                        gauge,
                        is_turntable: false,
                        start_connected,
                        end_connected,
                    });
                }
            }
        }
    }

    // Pass 2: Standalone, turnouts, crossings & turntables
    for (vi, p) in visible.iter().enumerate() {
        if visited_two[vi] {
            continue;
        }
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let lh = layer_info.get(lid).map(|x| x.0).unwrap_or(0.0);
        let e0 = piece_world_elev(p, lh, 0);
        let e1 = piece_world_elev(p, lh, 1);
        let entry_meta = find_entry(catalogs, &p.catalog_id);
        let is_road = is_road_id(catalogs, &p.catalog_id);
        let gauge = entry_meta.and_then(|e| e.gauge).unwrap_or(16.5);
        let half_w = if is_road {
            entry_meta.and_then(|e| e.road_width).unwrap_or(60.0) as f32 * 0.5
        } else {
            ((gauge / 16.5) * 20.0) as f32
        };
        let has_catenary = p.has_catenary == Some(true);
        let kind = entry_meta.map(|e| e.kind.as_str());
        if kind == Some("turntable") {
            let outer_r = entry_meta
                .and_then(|e| e.radius.or_else(|| e.length.map(|l| l * 0.5)))
                .unwrap_or(189.0);
            out.push(CorridorChain {
                pts_xze: vec![[p.transform.x as f32, p.transform.y as f32, e0 as f32]],
                half_w: (outer_r + 4.0) as f32,
                is_road: false,
                has_catenary,
                gauge,
                is_turntable: true,
                start_connected: true,
                end_connected: true,
            });
            continue;
        }
        if kind == Some("transfer-table") {
            let l = entry_meta.and_then(|e| e.length).unwrap_or(360.0);
            let zs = crate::turntables_3d::extract_transfer_track_zs(entry_meta);
            let min_tz = zs[0] as f64;
            let max_tz = *zs.last().unwrap_or(&309.6) as f64;
            let top_y = min_tz - 55.2;
            let bot_y = max_tz + 55.2;
            let mut pts_xze = Vec::new();
            for xi in 0..=6 {
                let lx = 30.0 + (l - 60.0) * (xi as f64 / 6.0);
                for yi in 0..=6 {
                    let ly = (top_y + 30.0) + ((bot_y - top_y) - 60.0) * (yi as f64 / 6.0);
                    let rp = rotate(Vec2 { x: lx, y: ly }, p.transform.rotation);
                    pts_xze.push([
                        (p.transform.x + rp.x) as f32,
                        (p.transform.y + rp.y) as f32,
                        e0 as f32,
                    ]);
                }
            }
            out.push(CorridorChain {
                pts_xze,
                half_w: 56.0,
                is_road: false,
                has_catenary,
                gauge,
                is_turntable: true,
                start_connected: true,
                end_connected: true,
            });
            continue;
        }
        let is_turntable = p.connectors.len() >= 6;
        let c = &p.connectors;

        let push_branch = |out: &mut Vec<CorridorChain>, a_idx: usize, b_idx: usize| {
            let ea = piece_world_elev(p, lh, a_idx);
            let eb = piece_world_elev(p, lh, b_idx);
            let pts2d = sample_arc_between_connectors(
                c[a_idx].x,
                c[a_idx].y,
                c[a_idx].angle,
                c[b_idx].x,
                c[b_idx].y,
                c[b_idx].angle,
                16,
            );
            let n = pts2d.len().saturating_sub(1).max(1);
            let raw: Vec<[f32; 3]> = pts2d
                .into_iter()
                .enumerate()
                .map(|(k, pt)| {
                    let u = k as f64 / n as f64;
                    [pt.x as f32, pt.y as f32, (ea + (eb - ea) * u) as f32]
                })
                .collect();
            let mut dense = Vec::new();
            push_dense_subdivided(&mut dense, &raw);
            if dense.len() >= 2 {
                out.push(CorridorChain {
                    pts_xze: dense,
                    half_w,
                    is_road,
                    has_catenary,
                    gauge,
                    is_turntable,
                    start_connected: c[a_idx].connected_to.is_some(),
                    end_connected: c[b_idx].connected_to.is_some(),
                });
            }
        };

        match c.len() {
            0 => {}
            1 => {
                let rad = deg2rad(c[0].angle + 180.0);
                let raw = vec![
                    [c[0].x as f32, c[0].y as f32, e0 as f32],
                    [
                        (c[0].x + rad.cos() * 70.0) as f32,
                        (c[0].y + rad.sin() * 70.0) as f32,
                        e1 as f32,
                    ],
                ];
                let mut dense = Vec::new();
                push_dense_subdivided(&mut dense, &raw);
                out.push(CorridorChain {
                    pts_xze: dense,
                    half_w,
                    is_road,
                    has_catenary,
                    gauge,
                    is_turntable: false,
                    start_connected: c[0].connected_to.is_some(),
                    end_connected: false,
                });
            }
            2 => push_branch(&mut out, 0, 1),
            3 => {
                push_branch(&mut out, 0, 1);
                push_branch(&mut out, 0, 2);
            }
            4 => {
                let a0 = c[0].angle.to_radians();
                let v0x = a0.cos();
                let v0y = a0.sin();
                let facing_opposite = c[1..4]
                    .iter()
                    .filter(|ci| {
                        let ai = ci.angle.to_radians();
                        v0x * ai.cos() + v0y * ai.sin() < -0.15
                    })
                    .count();
                if facing_opposite == 3 {
                    push_branch(&mut out, 0, 1);
                    push_branch(&mut out, 0, 2);
                    push_branch(&mut out, 0, 3);
                } else {
                    push_branch(&mut out, 0, 1);
                    push_branch(&mut out, 2, 3);
                }
            }
            _ => {
                let half = c.len() / 2;
                push_branch(&mut out, 0, half);
            }
        }
    }

    out
}

#[derive(Debug, Clone, Copy)]
pub struct LocalTunnelPt {
    pub lx: f64,
    pub lz: f64,
    pub rel_elev: f64,
    pub norm_r: f64,
}

#[derive(Debug, Clone)]
pub struct TunnelPortalInfo {
    pub at: LocalTunnelPt,
    pub dir_xz: [f64; 2], // unit vector pointing INTO the mountain
    pub prof: TunnelProfile,
    pub suppressed_by_overlap: bool,
}

#[derive(Debug, Clone)]
pub struct TunnelRun {
    pub prof: TunnelProfile,
    pub pts: Vec<LocalTunnelPt>,
    pub start_portal: Option<TunnelPortalInfo>,
    pub end_portal: Option<TunnelPortalInfo>,
}

pub(super) fn bisect_boundary_crossing(
    a: LocalTunnelPt,
    b: LocalTunnelPt,
    r_portal: f64,
    mountain: &Model3D,
) -> LocalTunnelPt {
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    let a_inside = a.norm_r <= r_portal;
    for _ in 0..10 {
        let t = (lo + hi) * 0.5;
        let lx = a.lx + (b.lx - a.lx) * t;
        let lz = a.lz + (b.lz - a.lz) * t;
        let (nr, _) = evaluate_mountain_local(lx, lz, mountain);
        if (nr <= r_portal) == a_inside {
            lo = t;
        } else {
            hi = t;
        }
    }
    let t = (lo + hi) * 0.5;
    let lx = a.lx + (b.lx - a.lx) * t;
    let lz = a.lz + (b.lz - a.lz) * t;
    let (norm_r, _) = evaluate_mountain_local(lx, lz, mountain);
    LocalTunnelPt {
        lx,
        lz,
        rel_elev: a.rel_elev + (b.rel_elev - a.rel_elev) * t,
        norm_r,
    }
}

pub(super) fn inward_dir_local(pts: &[LocalTunnelPt], at_start: bool) -> [f64; 2] {
    let base = if at_start { pts[0] } else { *pts.last().unwrap() };
    let iter_pts: Box<dyn Iterator<Item = &LocalTunnelPt>> = if at_start {
        Box::new(pts.iter().skip(1))
    } else {
        Box::new(pts.iter().rev().skip(1))
    };
    for p in iter_pts {
        let dx = p.lx - base.lx;
        let dz = p.lz - base.lz;
        let len = dx.hypot(dz);
        if len >= 4.0 {
            return [dx / len, dz / len];
        }
    }
    let other = if at_start { *pts.last().unwrap() } else { pts[0] };
    let dx = other.lx - base.lx;
    let dz = other.lz - base.lz;
    let len = dx.hypot(dz).max(1e-6);
    [dx / len, dz / len]
}

pub(super) struct PortalCandidate {
    run_idx: usize,
    is_start: bool,
    at: LocalTunnelPt,
    dir_xz: [f64; 2],
    is_road: bool,
    has_catenary: bool,
    gauge: f64,
    prof: TunnelProfile,
    suppressed_by_overlap: bool,
}

/// Extracts contiguous tunnel spans inside `mountain`, handles boundary crossings,
/// suppresses overlap portals inside neighboring mountains, and clusters parallel multi-track portals.
pub fn compute_tunnel_runs_for_mountain(
    mountain: &Model3D,
    m_base_elev: f64,
    chains: &[CorridorChain],
    all_mountains: &[&Model3D],
    layer_info: &HashMap<&str, (f64, bool)>,
) -> Vec<TunnelRun> {
    let sc = mountain.scale.unwrap_or(1.0).max(0.05);
    let mh = mountain.height.max(20.0) * sc;
    let bound_r = (mountain.width.max(mountain.depth) * sc) * 0.65;
    let cx = mountain.transform.x;
    let cz = mountain.transform.y;

    let mut raw_runs: Vec<(TunnelRun, &CorridorChain)> = Vec::new();

    for ch in chains {
        let near = ch
            .pts_xze
            .iter()
            .any(|p| (p[0] as f64 - cx).hypot(p[1] as f64 - cz) <= bound_r + 120.0);
        if !near || ch.pts_xze.len() < 2 {
            continue;
        }

        let prof = create_tunnel_profile(ch.is_road, ch.has_catenary, ch.gauge, 1, 0.0);
        let mut local_pts: Vec<LocalTunnelPt> = Vec::new();
        for p in &ch.pts_xze {
            let rel_elev = p[2] as f64 - m_base_elev;
            if rel_elev < -35.0 || rel_elev > mh - 10.0 {
                continue;
            }
            let (lx, lz) = world_to_mountain_local(p[0] as f64, p[1] as f64, mountain);
            let (norm_r, _) = evaluate_mountain_local(lx, lz, mountain);
            local_pts.push(LocalTunnelPt {
                lx,
                lz,
                rel_elev,
                norm_r,
            });
        }
        if local_pts.len() < 2 || !local_pts.iter().any(|p| p.norm_r <= ELLIPSE_TOLERANCE) {
            continue;
        }

        struct Span {
            pts: Vec<LocalTunnelPt>,
            has_start_portal: bool,
            has_end_portal: bool,
        }
        let mut spans: Vec<Span> = Vec::new();
        let mut cur: Option<Span> = None;

        for (i, &p) in local_pts.iter().enumerate() {
            if p.norm_r <= R_PORTAL {
                if cur.is_none() {
                    let mut pts = Vec::new();
                    let has_start = if i > 0 {
                        pts.push(bisect_boundary_crossing(
                            local_pts[i - 1],
                            p,
                            R_PORTAL,
                            mountain,
                        ));
                        true
                    } else {
                        !ch.start_connected && p.norm_r >= R_PORTAL - 0.25
                    };
                    cur = Some(Span {
                        pts,
                        has_start_portal: has_start,
                        has_end_portal: false,
                    });
                }
                cur.as_mut().unwrap().pts.push(p);
            } else if let Some(mut s) = cur.take() {
                s.pts.push(bisect_boundary_crossing(
                    local_pts[i - 1],
                    p,
                    R_PORTAL,
                    mountain,
                ));
                s.has_end_portal = true;
                if s.pts.len() >= 2 {
                    spans.push(s);
                }
            }
        }
        if let Some(mut s) = cur.take() {
            if let Some(last) = s.pts.last() {
                if !ch.end_connected && last.norm_r >= R_PORTAL - 0.25 {
                    s.has_end_portal = true;
                }
            }
            if s.pts.len() >= 2 {
                spans.push(s);
            }
        }

        for sp in spans {
            raw_runs.push((
                TunnelRun {
                    prof,
                    pts: sp.pts,
                    start_portal: if sp.has_start_portal {
                        Some(TunnelPortalInfo {
                            at: LocalTunnelPt { lx: 0.0, lz: 0.0, rel_elev: 0.0, norm_r: 0.0 },
                            dir_xz: [0.0, 1.0],
                            prof,
                            suppressed_by_overlap: false,
                        })
                    } else {
                        None
                    },
                    end_portal: if sp.has_end_portal {
                        Some(TunnelPortalInfo {
                            at: LocalTunnelPt { lx: 0.0, lz: 0.0, rel_elev: 0.0, norm_r: 0.0 },
                            dir_xz: [0.0, 1.0],
                            prof,
                            suppressed_by_overlap: false,
                        })
                    } else {
                        None
                    },
                },
                ch,
            ));
        }
    }

    if raw_runs.is_empty() {
        return Vec::new();
    }

    let mut runs: Vec<TunnelRun> = raw_runs.iter().map(|(r, _)| r.clone()).collect();
    let mut candidates: Vec<PortalCandidate> = Vec::new();

    for (run_idx, (run, ch)) in raw_runs.iter().enumerate() {
        if run.start_portal.is_some() {
            let at = run.pts[0];
            if at.rel_elev >= -35.0 && at.rel_elev <= mh - 10.0 {
                let (wx, wy) = mountain_local_to_world(at.lx, at.lz, Some(mountain));
                let suppressed = is_point_inside_other_mountain(
                    wx,
                    wy,
                    m_base_elev + at.rel_elev,
                    &mountain.id,
                    all_mountains,
                    layer_info,
                );
                candidates.push(PortalCandidate {
                    run_idx,
                    is_start: true,
                    at,
                    dir_xz: inward_dir_local(&run.pts, true),
                    is_road: ch.is_road,
                    has_catenary: ch.has_catenary,
                    gauge: ch.gauge,
                    prof: run.prof,
                    suppressed_by_overlap: suppressed,
                });
            } else {
                runs[run_idx].start_portal = None;
            }
        }
        if run.end_portal.is_some() {
            let at = *run.pts.last().unwrap();
            if at.rel_elev >= -35.0 && at.rel_elev <= mh - 10.0 {
                let (wx, wy) = mountain_local_to_world(at.lx, at.lz, Some(mountain));
                let suppressed = is_point_inside_other_mountain(
                    wx,
                    wy,
                    m_base_elev + at.rel_elev,
                    &mountain.id,
                    all_mountains,
                    layer_info,
                );
                candidates.push(PortalCandidate {
                    run_idx,
                    is_start: false,
                    at,
                    dir_xz: inward_dir_local(&run.pts, false),
                    is_road: ch.is_road,
                    has_catenary: ch.has_catenary,
                    gauge: ch.gauge,
                    prof: run.prof,
                    suppressed_by_overlap: suppressed,
                });
            } else {
                runs[run_idx].end_portal = None;
            }
        }
    }

    // Cluster nearby parallel rail portals into single wide multi-track portals
    let mut grouped = vec![false; candidates.len()];
    for i in 0..candidates.len() {
        if grouped[i] {
            continue;
        }
        grouped[i] = true;
        let mut cluster = vec![i];

        if !candidates[i].is_road {
            let g_scale = (candidates[i].gauge / 16.5).clamp(0.4, 3.0);
            let max_group_dist = 115.0 * g_scale;
            for j in (i + 1)..candidates.len() {
                if grouped[j]
                    || candidates[j].is_road
                    || candidates[j].run_idx == candidates[i].run_idx
                {
                    continue;
                }
                let ci = &candidates[i];
                let cj = &candidates[j];
                let dist = (ci.at.lx - cj.at.lx).hypot(ci.at.lz - cj.at.lz);
                let dot = ci.dir_xz[0] * cj.dir_xz[0] + ci.dir_xz[1] * cj.dir_xz[1];
                let elev_diff = (ci.at.rel_elev - cj.at.rel_elev).abs();
                if dist <= max_group_dist && dot > 0.5 && elev_diff < 25.0 {
                    grouped[j] = true;
                    cluster.push(j);
                }
            }
        }

        if cluster.len() == 1 {
            let cand = &candidates[cluster[0]];
            let pinfo = TunnelPortalInfo {
                at: cand.at,
                dir_xz: cand.dir_xz,
                prof: cand.prof,
                suppressed_by_overlap: cand.suppressed_by_overlap,
            };
            if cand.is_start {
                runs[cand.run_idx].start_portal = Some(pinfo);
            } else {
                runs[cand.run_idx].end_portal = Some(pinfo);
            }
        } else {
            let k = cluster.len();
            let kf = k as f64;
            let has_cat = cluster
                .iter()
                .any(|&idx| candidates[idx].has_catenary || candidates[idx].prof.has_catenary);
            let gauge = candidates[cluster[0]].gauge;
            let suppressed = cluster.iter().all(|&idx| candidates[idx].suppressed_by_overlap);

            let avg_lx = cluster.iter().map(|&idx| candidates[idx].at.lx).sum::<f64>() / kf;
            let avg_lz = cluster.iter().map(|&idx| candidates[idx].at.lz).sum::<f64>() / kf;
            let avg_elev = cluster
                .iter()
                .map(|&idx| candidates[idx].at.rel_elev)
                .sum::<f64>()
                / kf;
            let sum_dx = cluster.iter().map(|&idx| candidates[idx].dir_xz[0]).sum::<f64>();
            let sum_dz = cluster.iter().map(|&idx| candidates[idx].dir_xz[1]).sum::<f64>();
            let dlen = sum_dx.hypot(sum_dz).max(1e-6);
            let comb_dir = [sum_dx / dlen, sum_dz / dlen];

            let lat_x = -comb_dir[1];
            let lat_z = comb_dir[0];
            let (mut min_lat, mut max_lat) = (f64::INFINITY, f64::NEG_INFINITY);
            for &idx in &cluster {
                let c_pt = candidates[idx].at;
                let lat = (c_pt.lx - avg_lx) * lat_x + (c_pt.lz - avg_lz) * lat_z;
                min_lat = min_lat.min(lat);
                max_lat = max_lat.max(lat);
            }
            let spread = (max_lat - min_lat).max(0.0);
            let multi_prof = create_tunnel_profile(false, has_cat, gauge, k, spread);
            let (nr, _) = evaluate_mountain_local(avg_lx, avg_lz, mountain);
            let combined_portal = TunnelPortalInfo {
                at: LocalTunnelPt {
                    lx: avg_lx,
                    lz: avg_lz,
                    rel_elev: avg_elev,
                    norm_r: nr,
                },
                dir_xz: comb_dir,
                prof: multi_prof,
                suppressed_by_overlap: suppressed,
            };

            for &idx in &cluster {
                let cand = &candidates[idx];
                if cand.is_start {
                    runs[cand.run_idx].start_portal = Some(combined_portal.clone());
                } else {
                    runs[cand.run_idx].end_portal = Some(combined_portal.clone());
                }
            }
        }
    }

    runs
}

/// Counts intersecting tunnel spans for UI Inspector status display.
pub fn count_mountain_tunnel_collisions(
    mountain: &Model3D,
    layout: &Layout,
    catalogs: &[Catalog],
) -> usize {
    let layer_info = layer_height_visible_map(layout);
    let chains = build_connected_corridor_chains(layout, catalogs, &layer_info);
    let m_lid = mountain.layer_id.as_deref().unwrap_or("layer-1");
    let m_lh = layer_info.get(m_lid).map(|x| x.0).unwrap_or(0.0);
    let m_base = m_lh + mountain.elevation.unwrap_or(0.0);
    let all_mountains: Vec<&Model3D> = layout
        .models
        .as_ref()
        .map(|ms| {
            ms.iter()
                .filter(|m| m.builtin_id.as_deref() == Some("mountain"))
                .collect()
        })
        .unwrap_or_default();
    compute_tunnel_runs_for_mountain(mountain, m_base, &chains, &all_mountains, &layer_info).len()
}

