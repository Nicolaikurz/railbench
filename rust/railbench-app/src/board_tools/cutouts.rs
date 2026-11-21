use super::*;

pub(super) fn piece_centerline_clipped(piece: &TrackPiece, from_c: usize, to_c: usize, t0: f64, t1: f64) -> Vec<Vec2> {
    if let (Some(offsets), Some(chord)) = (&piece.custom_offsets, piece.custom_chord_dist) {
        if offsets.len() >= 3 {
            let flat = sample_flex_spline_points(chord, offsets[0], offsets[1], offsets[2], 16);
            let n = flat.len() / 5;
            let mut pts = Vec::new();
            for k in 0..n {
                let u = k as f64 / (n.saturating_sub(1).max(1)) as f64;
                if u + 0.02 < t0 || u - 0.02 > t1 {
                    continue;
                }
                let rp = rotate(Vec2 { x: flat[k * 5], y: flat[k * 5 + 1] }, piece.transform.rotation);
                pts.push(Vec2 {
                    x: piece.transform.x + rp.x,
                    y: piece.transform.y + rp.y,
                });
            }
            if pts.len() >= 2 {
                return pts;
            }
        }
    }

    let full = if piece.connectors.len() >= 2 {
        let c0 = piece.connectors.get(from_c).unwrap_or(&piece.connectors[0]);
        let c1 = piece.connectors.get(to_c).unwrap_or(&piece.connectors[piece.connectors.len() - 1]);
        sample_arc_between_connectors(c0.x, c0.y, c0.angle, c1.x, c1.y, c1.angle, 12)
    } else {
        vec![Vec2 {
            x: piece.transform.x,
            y: piece.transform.y,
        }]
    };
    if full.len() < 2 {
        return full;
    }

    // Arc-length clip between t0 and t1
    let mut lens = vec![0.0_f64; full.len()];
    for i in 1..full.len() {
        lens[i] = lens[i - 1] + (full[i].x - full[i - 1].x).hypot(full[i].y - full[i - 1].y);
    }
    let total = *lens.last().unwrap_or(&0.0);
    if total < 1e-4 {
        return full;
    }
    let d0 = t0.clamp(0.0, 1.0) * total;
    let d1 = t1.clamp(t0, 1.0) * total;
    let sample_at = |d: f64| -> Vec2 {
        for i in 1..full.len() {
            if lens[i] >= d - 1e-6 || i + 1 == full.len() {
                let span = (lens[i] - lens[i - 1]).max(1e-9);
                let u = ((d - lens[i - 1]) / span).clamp(0.0, 1.0);
                return Vec2 {
                    x: full[i - 1].x + (full[i].x - full[i - 1].x) * u,
                    y: full[i - 1].y + (full[i].y - full[i - 1].y) * u,
                };
            }
        }
        full[full.len() - 1]
    };

    let steps = 14usize;
    let mut out = Vec::with_capacity(steps + 1);
    for k in 0..=steps {
        let u = k as f64 / steps as f64;
        out.push(sample_at(d0 + (d1 - d0) * u));
    }
    out
}

pub fn compute_table_track_cutouts(
    layout: &Layout,
    catalogs: &[Catalog],
    board: &BaseboardConfig,
    deck_elev: f64,
    deck_thickness: f64,
) -> Vec<TrackCutoutPolygon> {
    if !board.enabled || layout.pieces.is_empty() {
        return Vec::new();
    }
    let bounds = get_baseboard_bounds(board);
    let clearance = min_clearance_for_scale(&layout.scale);
    let mut layer_map: HashMap<&str, &Layer> = HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            layer_map.insert(l.id.as_str(), l);
        }
    }

    let mut qualifying: Vec<&TrackPiece> = Vec::new();
    for p in &layout.pieces {
        let p_clearance = piece_vertical_clearance(p, &layout.scale, deck_thickness);
        let (passes, _, _) = piece_passes_through_deck(p, &layer_map, p_clearance, deck_elev, deck_thickness);
        if !passes {
            continue;
        }
        let near = is_point_inside_baseboard(Vec2 { x: p.transform.x, y: p.transform.y }, board)
            || p.connectors
                .iter()
                .any(|c| is_point_inside_baseboard(Vec2 { x: c.x, y: c.y }, board))
            || (p.transform.x >= bounds.x - 50.0
                && p.transform.x <= bounds.x + bounds.width + 50.0
                && p.transform.y >= bounds.y - 50.0
                && p.transform.y <= bounds.y + bounds.depth + 50.0);
        if near {
            qualifying.push(p);
        }
    }
    if qualifying.is_empty() {
        return Vec::new();
    }

    // Build adjacency among qualifying pieces
    let n_q = qualifying.len();
    let conn_height = |p: &TrackPiece, c_idx: usize| -> f64 {
        let base = layer_map
            .get(p.layer_id.as_deref().unwrap_or("layer-1"))
            .and_then(|l| l.height_mm)
            .unwrap_or(0.0);
        let local = p.elevation.map_or(0.0, |e| if c_idx == 0 { e.start } else { e.end });
        base + local
    };
    let mut adj: Vec<Vec<(usize, usize, usize)>> = vec![Vec::new(); n_q];
    for i in 0..n_q {
        for j in (i + 1)..n_q {
            for (ca_i, ca) in qualifying[i].connectors.iter().enumerate() {
                let ha = conn_height(qualifying[i], ca_i);
                for (cb_i, cb) in qualifying[j].connectors.iter().enumerate() {
                    let hb = conn_height(qualifying[j], cb_i);
                    if (ca.x - cb.x).hypot(ca.y - cb.y) <= 8.0 && (ha - hb).abs() <= 5.0 {
                        adj[i].push((j, ca_i, cb_i));
                        adj[j].push((i, cb_i, ca_i));
                    }
                }
            }
        }
    }

    let mut unvisited: Vec<bool> = vec![true; n_q];
    let mut cutouts = Vec::new();
    let b_min_x = bounds.x + 6.0;
    let b_max_x = bounds.x + bounds.width - 6.0;
    let b_min_y = bounds.y + 6.0;
    let b_max_y = bounds.y + bounds.depth - 6.0;

    while let Some(start) = {
        let mut best_s = None;
        let mut min_deg = usize::MAX;
        for (i, &unv) in unvisited.iter().enumerate() {
            if !unv {
                continue;
            }
            let deg = adj[i].iter().filter(|e| unvisited[e.0]).count();
            if deg < min_deg {
                min_deg = deg;
                best_s = Some(i);
                if deg <= 1 {
                    break;
                }
            }
        }
        best_s
    } {
        let mut chain: Vec<(usize, usize, usize)> = Vec::new();
        let mut cur = Some(start);
        let mut prev_idx: Option<usize> = None;

        while let Some(ci) = cur {
            unvisited[ci] = false;
            let (from_c, to_c) = if let Some(pi) = prev_idx {
                if let Some(&(_, my_c, _)) = adj[ci].iter().find(|e| e.0 == pi) {
                    (my_c, if my_c == 0 { 1 } else { 0 })
                } else {
                    (0, 1)
                }
            } else if let Some(&(_, my_c, _)) = adj[ci].iter().find(|e| unvisited[e.0]) {
                (if my_c == 0 { 1 } else { 0 }, my_c)
            } else if let Some(&(_, my_c, _)) = adj[ci].first() {
                (if my_c == 0 { 1 } else { 0 }, my_c)
            } else {
                (0, 1)
            };

            chain.push((ci, from_c, to_c));
            prev_idx = Some(ci);
            cur = adj[ci]
                .iter()
                .find(|e| e.1 == to_c && unvisited[e.0])
                .map(|e| e.0);
        }

        let mut centerline: Vec<Vec2> = Vec::new();
        let mut max_w = 0.0_f64;
        let chain_has_catenary = chain
            .iter()
            .any(|&(qi, _, _)| piece_has_catenary_overhead(qualifying[qi]));
        let chain_clearance = if chain_has_catenary {
            clearance.max(deck_thickness + 98.0)
        } else {
            clearance
        };
        let min_cut_elev = deck_elev - chain_clearance;
        let max_cut_elev = deck_elev;
        let mut start_elev = f64::NAN;
        let mut end_elev = f64::NAN;

        for (qi, from_c, to_c) in chain {
            let p = qualifying[qi];
            let entry = find_entry(catalogs, &p.catalog_id);
            let mut pw = get_piece_clearance_width(p, entry);
            if is_road_id(catalogs, &p.catalog_id) && entry.is_none() {
                pw = 76.0;
            }
            max_w = max_w.max(pw);

            let base_h = layer_map
                .get(p.layer_id.as_deref().unwrap_or("layer-1"))
                .and_then(|l| l.height_mm)
                .unwrap_or(0.0);
            let ps = p.elevation.map_or(0.0, |e| e.start);
            let pe = p.elevation.map_or(ps, |e| e.end);
            let h0 = base_h + ps;
            let h1 = base_h + pe;
            let h_from = if from_c == 0 { h0 } else { h1 };
            let h_to = if to_c == 0 { h0 } else { h1 };
            let dh = h_to - h_from;
            let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
            if dh.abs() >= 0.5 {
                let ta = (max_cut_elev - h_from) / dh;
                let tb = (min_cut_elev - h_from) / dh;
                t0 = ta.min(tb).clamp(0.0, 1.0);
                t1 = ta.max(tb).clamp(0.0, 1.0);
            }
            if t1 <= t0 + 0.01 {
                continue;
            }
            if start_elev.is_nan() {
                start_elev = h_from + dh * t0;
            }
            end_elev = h_from + dh * t1;
            for pt in piece_centerline_clipped(p, from_c, to_c, t0, t1) {
                if centerline
                    .last()
                    .map_or(true, |last| (pt.x - last.x).hypot(pt.y - last.y) >= 1.0)
                {
                    centerline.push(pt);
                }
            }
        }

        if centerline.len() < 2 {
            continue;
        }
        // Extend the deeper subterranean end longitudinally so borderline catenary masts clear the end wall.
        let end_margin = if chain_has_catenary { 14.0 } else { 4.0 };
        if !start_elev.is_nan() && !end_elev.is_nan() {
            if start_elev <= end_elev {
                let p0 = centerline[0];
                let p1 = centerline[1];
                let len = (p0.x - p1.x).hypot(p0.y - p1.y).max(1e-6);
                centerline[0] = Vec2 {
                    x: p0.x + ((p0.x - p1.x) / len) * end_margin,
                    y: p0.y + ((p0.y - p1.y) / len) * end_margin,
                };
            }
            if end_elev <= start_elev {
                let last_i = centerline.len() - 1;
                let pn = centerline[last_i];
                let pp = centerline[last_i - 1];
                let len = (pn.x - pp.x).hypot(pn.y - pp.y).max(1e-6);
                centerline[last_i] = Vec2 {
                    x: pn.x + ((pn.x - pp.x) / len) * end_margin,
                    y: pn.y + ((pn.y - pp.y) / len) * end_margin,
                };
            }
        }

        let half_w = max_w * 0.5;
        let m_len = centerline.len();
        let mut left_pts = Vec::with_capacity(m_len);
        let mut right_pts = Vec::with_capacity(m_len);
        let seg_unit_norm = |a: Vec2, b: Vec2| -> (f64, f64) {
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let len = dx.hypot(dy).max(1e-6);
            (-dy / len, dx / len)
        };

        for i in 0..m_len {
            let cur_pt = centerline[i];
            let (nx, ny, mscale) = if i == 0 {
                let (nx, ny) = seg_unit_norm(cur_pt, centerline[1]);
                (nx, ny, 1.0)
            } else if i + 1 == m_len {
                let (nx, ny) = seg_unit_norm(centerline[m_len - 2], cur_pt);
                (nx, ny, 1.0)
            } else {
                let (n0x, n0y) = seg_unit_norm(centerline[i - 1], cur_pt);
                let (n1x, n1y) = seg_unit_norm(cur_pt, centerline[i + 1]);
                let mx = n0x + n1x;
                let my = n0y + n1y;
                let mlen = mx.hypot(my);
                if mlen < 1e-4 {
                    (n1x, n1y, 1.0)
                } else {
                    let ux = mx / mlen;
                    let uy = my / mlen;
                    let cos_half = (ux * n1x + uy * n1y).clamp(0.68, 1.0);
                    (ux, uy, (1.0 / cos_half).clamp(1.0, 1.45))
                }
            };
            let eff_hw = half_w * mscale;
            left_pts.push(Vec2 {
                x: cur_pt.x + nx * eff_hw,
                y: cur_pt.y + ny * eff_hw,
            });
            right_pts.push(Vec2 {
                x: cur_pt.x - nx * eff_hw,
                y: cur_pt.y - ny * eff_hw,
            });
        }

        let mut raw_flat = Vec::with_capacity((left_pts.len() + right_pts.len()) * 2);
        for p in &left_pts {
            raw_flat.push(p.x);
            raw_flat.push(p.y);
        }
        for p in right_pts.iter().rev() {
            raw_flat.push(p.x);
            raw_flat.push(p.y);
        }

        let sim = simplify_cutout_polygon(&raw_flat, b_min_x, b_max_x, b_min_y, b_max_y, 3.0, 0.04);
        if sim.len() >= 6 {
            let pts: Vec<Vec2> = sim
                .chunks_exact(2)
                .map(|c| Vec2 { x: c[0], y: c[1] })
                .collect();
            if signed_polygon_area(&pts).abs() >= 10.0 {
                cutouts.push(TrackCutoutPolygon { points: pts });
            }
        }
    }

    cutouts
}

pub fn is_point_inside_cutouts(
    pt: Vec2,
    cutouts: &[TrackCutoutPolygon],
    safety_buffer: f64,
) -> bool {
    for c in cutouts {
        if c.points.len() < 3 {
            continue;
        }
        if is_point_in_polygon(pt, &c.points) {
            return true;
        }
        let n = c.points.len();
        for i in 0..n {
            if dist_to_segment(pt, c.points[i], c.points[(i + 1) % n]) <= safety_buffer {
                return true;
            }
        }
    }
    false
}

