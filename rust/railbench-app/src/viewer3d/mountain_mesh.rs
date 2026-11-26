use super::*;

pub(super) fn mountain_biome_color(h_ratio: f32, style: Option<&str>) -> [f32; 3] {
    match style.unwrap_or("alpine") {
        "rocky" => {
            if h_ratio > 0.88 {
                [0.78, 0.78, 0.80]
            } else if h_ratio > 0.22 {
                [0.34, 0.33, 0.31]
            } else {
                [0.28, 0.34, 0.20]
            }
        }
        "forest" => {
            if h_ratio > 0.78 {
                [0.38, 0.36, 0.33]
            } else if h_ratio > 0.28 {
                [0.14, 0.28, 0.11]
            } else {
                [0.20, 0.35, 0.12]
            }
        }
        "meadow" => {
            if h_ratio > 0.65 {
                [0.32, 0.44, 0.16]
            } else {
                [0.24, 0.40, 0.13]
            }
        }
        _ => {
            if h_ratio > 0.82 {
                [0.96, 0.98, 1.00]
            } else if h_ratio > 0.35 {
                [0.36, 0.34, 0.32]
            } else {
                [0.24, 0.38, 0.12]
            }
        }
    }
}

// Tunnel portal & swept lining builders live in `crate::tunnels_3d`.

#[allow(clippy::too_many_arguments)]
pub(super) fn push_mountain_with_tunnels_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    m: &Model3D,
    y0: f32,
    layout: &Layout,
    boards: &[BaseboardConfig],
    chains: &[CorridorChain],
    all_mountains: &[&Model3D],
    layer_info: &std::collections::HashMap<&str, (f64, bool)>,
) {
    let scale = m.scale.unwrap_or(1.0).max(0.05) as f32;
    let w = (m.width as f32 * scale).max(60.0);
    let d = (m.depth as f32 * scale).max(60.0);
    let h = (m.height as f32 * scale).max(20.0);
    let cx = m.transform.x as f32;
    let cz = m.transform.y as f32;
    let rot_rad = (m.transform.rotation as f32).to_radians();
    let cos_f = (m.transform.rotation.to_radians()).cos();
    let sin_f = (m.transform.rotation.to_radians()).sin();
    let cos_inv = (-m.transform.rotation.to_radians()).cos();
    let sin_inv = (-m.transform.rotation.to_radians()).sin();

    let is_path = m.mountain_path.as_ref().map_or(false, |p| !p.is_empty());
    let nx = if is_path { 52usize } else { 40usize };
    let nz = if is_path { 52usize } else { 40usize };
    let cols = nx + 1;
    let rows = nz + 1;

    // Support polygon clipping from tables & layer wood sub-plates
    let support_polys = collect_terrain_support_polygons(layout, m.layer_id.as_deref(), boards);
    let clip_to_support = !support_polys.is_empty();
    let (poly_offsets, poly_coords) = crate::terrain_tools::pack_support_polygons(&support_polys);

    let mut heights = vec![0.0_f32; cols * rows];
    let mut inside_support = vec![true; cols * rows];
    let mut edge_clamped = vec![false; cols * rows];
    let mut norm_r_grid = vec![2.0_f32; cols * rows];
    let mut wx_grid = vec![0.0_f64; cols * rows];
    let mut wz_grid = vec![0.0_f64; cols * rows];

    let hw_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let poly_offsets_ref = &poly_offsets;
    let poly_coords_ref = &poly_coords;

    if hw_threads > 1 && rows * cols >= 512 {
        let rows_per_chunk = rows.div_ceil(hw_threads);
        let chunk_elems = rows_per_chunk * cols;
        std::thread::scope(|s| {
            for (
                chunk_idx,
                ((((h_chunk, sup_chunk), nr_chunk), wx_chunk), wz_chunk),
            ) in heights
                .chunks_mut(chunk_elems)
                .zip(inside_support.chunks_mut(chunk_elems))
                .zip(norm_r_grid.chunks_mut(chunk_elems))
                .zip(wx_grid.chunks_mut(chunk_elems))
                .zip(wz_grid.chunks_mut(chunk_elems))
                .enumerate()
            {
                let row_start = chunk_idx * rows_per_chunk;
                s.spawn(move || {
                    for (local_r, ((((h_row, sup_row), nr_row), wx_row), wz_row)) in h_chunk
                        .chunks_mut(cols)
                        .zip(sup_chunk.chunks_mut(cols))
                        .zip(nr_chunk.chunks_mut(cols))
                        .zip(wx_chunk.chunks_mut(cols))
                        .zip(wz_chunk.chunks_mut(cols))
                        .enumerate()
                    {
                        let iz = row_start + local_r;
                        let lz = (iz as f64 / nz as f64 - 0.5) * d as f64;
                        for ix in 0..cols {
                            let lx = (ix as f64 / nx as f64 - 0.5) * w as f64;
                            let wx = cx as f64 + (lx * cos_f - lz * sin_f);
                            let wz = cz as f64 + (lx * sin_f + lz * cos_f);
                            wx_row[ix] = wx;
                            wz_row[ix] = wz;
                            if clip_to_support && !is_point_in_any_polygon(wx, wz, poly_offsets_ref, poly_coords_ref) {
                                sup_row[ix] = false;
                                h_row[ix] = 0.0;
                                continue;
                            }
                            let (nr, base_h) = evaluate_mountain_local(lx, lz, m);
                            nr_row[ix] = nr as f32;
                            h_row[ix] = base_h as f32;
                        }
                    }
                });
            }
        });
    } else {
        for iz in 0..rows {
            let lz = (iz as f64 / nz as f64 - 0.5) * d as f64;
            for ix in 0..cols {
                let lx = (ix as f64 / nx as f64 - 0.5) * w as f64;
                let v_idx = iz * cols + ix;
                let wx = cx as f64 + (lx * cos_f - lz * sin_f);
                let wz = cz as f64 + (lx * sin_f + lz * cos_f);
                wx_grid[v_idx] = wx;
                wz_grid[v_idx] = wz;
                if clip_to_support && !is_point_in_any_polygon(wx, wz, &poly_offsets, &poly_coords) {
                    inside_support[v_idx] = false;
                    heights[v_idx] = 0.0;
                    continue;
                }
                let (nr, base_h) = evaluate_mountain_local(lx, lz, m);
                norm_r_grid[v_idx] = nr as f32;
                heights[v_idx] = base_h as f32;
            }
        }
    }

    // Snap boundary outside vertices adjacent to inside vertices onto the table/plate edge
    // so cut mountain faces meet the table/plate border cleanly with zero overhang.
    if clip_to_support {
        for iz in 0..rows {
            for ix in 0..cols {
                let v_idx = iz * cols + ix;
                if inside_support[v_idx] {
                    continue;
                }
                let mut has_in_nb = false;
                'nb_mtn: for dz in -1isize..=1 {
                    for dx in -1isize..=1 {
                        if dx == 0 && dz == 0 {
                            continue;
                        }
                        let nx_i = ix as isize + dx;
                        let nz_i = iz as isize + dz;
                        if nx_i >= 0 && nx_i < cols as isize && nz_i >= 0 && nz_i < rows as isize {
                            if inside_support[nz_i as usize * cols + nx_i as usize] {
                                has_in_nb = true;
                                break 'nb_mtn;
                            }
                        }
                    }
                }
                if has_in_nb {
                    let clamped = crate::terrain_tools::clamp_point_to_support_polygons(
                        Vec2 { x: wx_grid[v_idx], y: wz_grid[v_idx] },
                        &support_polys,
                        &poly_offsets,
                        &poly_coords,
                    );
                    wx_grid[v_idx] = clamped.x;
                    wz_grid[v_idx] = clamped.y;
                    let (slx, slz) = crate::terrain_tools::world_to_mountain_local(clamped.x, clamped.y, m);
                    let (nr, base_h) = evaluate_mountain_local(slx, slz, m);
                    norm_r_grid[v_idx] = nr as f32;
                    heights[v_idx] = base_h as f32;
                    edge_clamped[v_idx] = true;
                }
            }
        }
    }

    // Tunnel runs & portal placement using connected chains & neighboring mountains
    let tunnel_runs =
        compute_tunnel_runs_for_mountain(m, y0 as f64, chains, all_mountains, layer_info);

    // Collect local collision points across intersecting chains to carve approach cuttings & ensure headroom
    struct LocalCorridor {
        prof: TunnelProfile,
        inner: f32,
        outer_inside: f32,
        outer_approach: f32,
        outer_inside_sq: f32,
        outer_approach_sq: f32,
        min_lx: f32,
        max_lx: f32,
        min_lz: f32,
        max_lz: f32,
        pts: Vec<[f32; 3]>, // [lx, lz, rel_elev]
        g_min_x: f32,
        g_min_z: f32,
        inv_cell: f32,
        grid_w: usize,
        grid_h: usize,
        cells: Vec<Vec<usize>>,
    }
    let bound_r = w.max(d) * 0.65;
    let near_r_sq = (bound_r + 120.0).powi(2);
    let mut prepared: Vec<LocalCorridor> = Vec::new();
    for ch in chains {
        let near = ch
            .pts_xze
            .iter()
            .any(|p| (p[0] - cx).powi(2) + (p[1] - cz).powi(2) <= near_r_sq);
        if !near {
            continue;
        }
        let prof = create_tunnel_profile(ch.is_road, ch.has_catenary, ch.gauge, 1, 0.0);
        let inner = prof.inner_width * 0.5 + prof.pier;
        let outer_inside = inner + 45.0;
        let outer_approach = inner + APPROACH_SLOPE as f32;
        let max_reach = outer_inside.max(outer_approach);
        let (mut min_lx, mut max_lx, mut min_lz, mut max_lz) =
            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
        let (mut pmin_x, mut pmax_x, mut pmin_z, mut pmax_z) =
            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
        let mut pts = Vec::new();
        for p in &ch.pts_xze {
            let rel_elev = p[2] - y0;
            if rel_elev < -35.0 || rel_elev > h - 10.0 {
                continue;
            }
            let dx = p[0] as f64 - cx as f64;
            let dz = p[1] as f64 - cz as f64;
            let lx = (dx * cos_inv - dz * sin_inv) as f32;
            let lz = (dx * sin_inv + dz * cos_inv) as f32;
            min_lx = min_lx.min(lx - max_reach);
            max_lx = max_lx.max(lx + max_reach);
            min_lz = min_lz.min(lz - max_reach);
            max_lz = max_lz.max(lz + max_reach);
            pmin_x = pmin_x.min(lx);
            pmax_x = pmax_x.max(lx);
            pmin_z = pmin_z.min(lz);
            pmax_z = pmax_z.max(lz);
            pts.push([lx, lz, rel_elev]);
        }
        if !pts.is_empty() {
            let (inv_cell, grid_w, grid_h, cells) = if pts.len() > 16 {
                let min_cell = (max_reach * 1.05 + 1.0).max(32.0);
                let span_x = (pmax_x - pmin_x).max(min_cell);
                let span_z = (pmax_z - pmin_z).max(min_cell);
                let cell = min_cell.max(span_x / 63.0).max(span_z / 63.0);
                let inv = 1.0 / cell;
                let gw = ((pmax_x - pmin_x) * inv).floor() as usize + 1;
                let gh = ((pmax_z - pmin_z) * inv).floor() as usize + 1;
                let mut grid = vec![Vec::new(); gw * gh];
                for (pi, pt) in pts.iter().enumerate() {
                    let gx = (((pt[0] - pmin_x) * inv).floor() as usize).min(gw - 1);
                    let gz = (((pt[1] - pmin_z) * inv).floor() as usize).min(gh - 1);
                    grid[gz * gw + gx].push(pi);
                }
                (inv, gw, gh, grid)
            } else {
                (0.0, 0, 0, Vec::new())
            };
            prepared.push(LocalCorridor {
                prof,
                inner,
                outer_inside,
                outer_approach,
                outer_inside_sq: outer_inside * outer_inside,
                outer_approach_sq: outer_approach * outer_approach,
                min_lx,
                max_lx,
                min_lz,
                max_lz,
                pts,
                g_min_x: pmin_x,
                g_min_z: pmin_z,
                inv_cell,
                grid_w,
                grid_h,
                cells,
            });
        }
    }

    if !prepared.is_empty() {
        let prepared_ref = &prepared;
        let inside_sup_ref = &inside_support;
        let edge_clamped_ref = &edge_clamped;
        let wx_grid_ref = &wx_grid;
        let wz_grid_ref = &wz_grid;
        let norm_r_ref = &norm_r_grid;
        let carve_mtn_row = |iz: usize, h_row: &mut [f32]| {
            let lz_nom = (iz as f32 / nz as f32 - 0.5) * d;
            for (ix, h_cell) in h_row.iter_mut().enumerate() {
                let v_idx = iz * cols + ix;
                if !inside_sup_ref[v_idx] && !edge_clamped_ref[v_idx] {
                    continue;
                }
                let (lx, lz) = if edge_clamped_ref[v_idx] {
                    let (slx, slz) = crate::terrain_tools::world_to_mountain_local(
                        wx_grid_ref[v_idx],
                        wz_grid_ref[v_idx],
                        m,
                    );
                    (slx as f32, slz as f32)
                } else {
                    ((ix as f32 / nx as f32 - 0.5) * w, lz_nom)
                };
                let r_dist = norm_r_ref[v_idx];
                if r_dist >= 0.999 {
                    continue;
                }
                let inside_portal = r_dist <= R_PORTAL as f32;
                let mut base_h = *h_cell;

                for col in prepared_ref {
                    if lx < col.min_lx || lx > col.max_lx || lz < col.min_lz || lz > col.max_lz {
                        continue;
                    }
                    let (outer, outer_sq) = if inside_portal {
                        (col.outer_inside, col.outer_inside_sq)
                    } else {
                        (col.outer_approach, col.outer_approach_sq)
                    };
                    let apply_pt = |pt: [f32; 3], base_h: &mut f32| {
                        let dx = lx - pt[0];
                        let dz = lz - pt[1];
                        let d_sq = dx * dx + dz * dz;
                        if d_sq > outer_sq {
                            return;
                        }
                        let dist = d_sq.sqrt();
                        let blend = 1.0 - smoothstep(col.inner as f64, outer as f64, dist as f64) as f32;
                        if inside_portal {
                            let target = pt[2] + col.prof.total_h + 16.0;
                            if *base_h < target {
                                *base_h += (target - *base_h) * blend;
                            }
                        } else {
                            let target = pt[2].max(0.0);
                            if *base_h > target {
                                *base_h += (target - *base_h) * blend;
                            }
                        }
                    };
                    if col.cells.is_empty() {
                        for &pt in &col.pts {
                            apply_pt(pt, &mut base_h);
                        }
                    } else {
                        let qcx = ((lx - col.g_min_x) * col.inv_cell).floor() as isize;
                        let qcz = ((lz - col.g_min_z) * col.inv_cell).floor() as isize;
                        let x0 = (qcx - 1).max(0);
                        let x1 = (qcx + 1).min(col.grid_w as isize - 1);
                        let z0 = (qcz - 1).max(0);
                        let z1 = (qcz + 1).min(col.grid_h as isize - 1);
                        for gz in z0..=z1 {
                            for gx in x0..=x1 {
                                for &pi in &col.cells[gz as usize * col.grid_w + gx as usize] {
                                    apply_pt(col.pts[pi], &mut base_h);
                                }
                            }
                        }
                    }
                }
                *h_cell = base_h;
            }
        };

        if hw_threads > 1 && rows * cols >= 512 {
            let rows_per_chunk = (rows + hw_threads - 1) / hw_threads;
            let chunk_elems = rows_per_chunk * cols;
            std::thread::scope(|s| {
                for (chunk_idx, chunk_slice) in heights.chunks_mut(chunk_elems).enumerate() {
                    let row_start = chunk_idx * rows_per_chunk;
                    s.spawn(move || {
                        for (local_r, row_slice) in chunk_slice.chunks_mut(cols).enumerate() {
                            carve_mtn_row(row_start + local_r, row_slice);
                        }
                    });
                }
            });
        } else {
            for (iz, row_slice) in heights.chunks_mut(cols).enumerate() {
                carve_mtn_row(iz, row_slice);
            }
        }
    }

    // Gather all portals (for entrance curtain quad culling + mesh generation)
    let mut all_portals = Vec::new();
    for tr in &tunnel_runs {
        if let Some(ref sp) = tr.start_portal {
            all_portals.push(sp.clone());
        }
        if let Some(ref ep) = tr.end_portal {
            all_portals.push(ep.clone());
        }
    }

    let is_entrance_curtain_quad = |i_a: usize, i_b: usize, i_c: usize, i_d: usize| -> bool {
        if all_portals.is_empty() {
            return false;
        }
        let coords = [
            ((i_a % cols) as f32 / nx as f32 - 0.5) * w,
            ((i_a / cols) as f32 / nz as f32 - 0.5) * d,
            heights[i_a],
            ((i_b % cols) as f32 / nx as f32 - 0.5) * w,
            ((i_b / cols) as f32 / nz as f32 - 0.5) * d,
            heights[i_b],
            ((i_c % cols) as f32 / nx as f32 - 0.5) * w,
            ((i_c / cols) as f32 / nz as f32 - 0.5) * d,
            heights[i_c],
            ((i_d % cols) as f32 / nx as f32 - 0.5) * w,
            ((i_d / cols) as f32 / nz as f32 - 0.5) * d,
            heights[i_d],
        ];
        let min_y = coords[2].min(coords[5]).min(coords[8]).min(coords[11]);
        let max_y = coords[2].max(coords[5]).max(coords[8]).max(coords[11]);
        let cx_q = (coords[0] + coords[3] + coords[6] + coords[9]) * 0.25;
        let cz_q = (coords[1] + coords[4] + coords[7] + coords[10]) * 0.25;
        let vpts = [
            [coords[0], coords[1]],
            [coords[3], coords[4]],
            [coords[6], coords[7]],
            [coords[9], coords[10]],
            [cx_q, cz_q],
        ];

        for po in &all_portals {
            let pe = po.at.rel_elev as f32;
            if min_y >= pe + po.prof.total_h || max_y <= pe - 2.0 {
                continue;
            }
            let (mut min_fwd, mut max_fwd, mut min_lat, mut max_lat) =
                (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
            let px = po.at.lx as f32;
            let pz = po.at.lz as f32;
            let dx_dir = po.dir_xz[0] as f32;
            let dz_dir = po.dir_xz[1] as f32;
            for [vx, vz] in vpts {
                let dx = vx - px;
                let dz = vz - pz;
                let fwd = dx * dx_dir + dz * dz_dir;
                let lat = dx * (-dz_dir) + dz * dx_dir;
                min_fwd = min_fwd.min(fwd);
                max_fwd = max_fwd.max(fwd);
                min_lat = min_lat.min(lat);
                max_lat = max_lat.max(lat);
            }
            let half_clear_w = po.prof.inner_width * 0.5 + 4.0;
            if max_lat >= -half_clear_w && min_lat <= half_clear_w && max_fwd >= -42.0 && min_fwd <= 52.0 {
                return true;
            }
        }
        false
    };

    let step_x = w / nx as f32;
    let step_z = d / nz as f32;
    let base_v = verts.len() as u32;

    for iz in 0..rows {
        for ix in 0..cols {
            let v_idx = iz * cols + ix;
            let mh = heights[v_idx];
            let hl = heights[iz * cols + ix.saturating_sub(1)];
            let hr = heights[iz * cols + (ix + 1).min(nx)];
            let hd = heights[iz.saturating_sub(1) * cols + ix];
            let hu = heights[(iz + 1).min(nz) * cols + ix];
            let normal = normalize3([(hl - hr) / (2.0 * step_x), 1.0, (hd - hu) / (2.0 * step_z)]);
            let ratio = (mh / h.max(1.0)).clamp(0.0, 1.0);
            let col = mountain_biome_color(ratio, m.mountain_style.as_deref());
            let (rnx, rnz) = rot_xz(normal[0], normal[2], rot_rad);
            verts.push(Vertex3D::new(
                [wx_grid[v_idx] as f32, y0 + mh + 0.5, wz_grid[v_idx] as f32],
                [rnx, normal[1], rnz],
                col,
                MAT_TERRAIN,
            ));
        }
    }

    let valid_mtn_tri = |ia: usize, ib: usize, ic: usize| -> bool {
        if !(inside_support[ia] || edge_clamped[ia])
            || !(inside_support[ib] || edge_clamped[ib])
            || !(inside_support[ic] || edge_clamped[ic])
        {
            return false;
        }
        if !clip_to_support {
            return true;
        }
        let tri_cx = (wx_grid[ia] + wx_grid[ib] + wx_grid[ic]) / 3.0;
        let tri_cz = (wz_grid[ia] + wz_grid[ib] + wz_grid[ic]) / 3.0;
        is_point_in_any_polygon(tri_cx, tri_cz, &poly_offsets, &poly_coords)
    };

    for iz in 0..nz {
        for ix in 0..nx {
            let i_a = iz * cols + ix;
            let i_b = i_a + 1;
            let i_c = (iz + 1) * cols + ix;
            let i_d = i_c + 1;
            let (ha, hb, hc, hd) = (heights[i_a], heights[i_b], heights[i_c], heights[i_d]);
            if ha <= 0.1 && hb <= 0.1 && hc <= 0.1 && hd <= 0.1 {
                continue;
            }
            if is_entrance_curtain_quad(i_a, i_b, i_c, i_d) {
                continue;
            }
            if (ha > 0.1 || hc > 0.1 || hb > 0.1) && valid_mtn_tri(i_a, i_c, i_b) {
                idxs.extend_from_slice(&[base_v + i_a as u32, base_v + i_c as u32, base_v + i_b as u32]);
                for &(u, v) in &[(i_a, i_c), (i_c, i_b), (i_b, i_a)] {
                    if edge_clamped[u] && edge_clamped[v] && heights[u].max(heights[v]) > 0.5 {
                        push_diorama_skirt_quad(
                            verts,
                            idxs,
                            wx_grid[u] as f32,
                            wz_grid[u] as f32,
                            y0 + heights[u] + 0.5,
                            wx_grid[v] as f32,
                            wz_grid[v] as f32,
                            y0 + heights[v] + 0.5,
                            y0,
                        );
                    }
                }
            }
            if (hb > 0.1 || hc > 0.1 || hd > 0.1) && valid_mtn_tri(i_b, i_c, i_d) {
                idxs.extend_from_slice(&[base_v + i_b as u32, base_v + i_c as u32, base_v + i_d as u32]);
                for &(u, v) in &[(i_b, i_c), (i_c, i_d), (i_d, i_b)] {
                    if edge_clamped[u] && edge_clamped[v] && heights[u].max(heights[v]) > 0.5 {
                        push_diorama_skirt_quad(
                            verts,
                            idxs,
                            wx_grid[u] as f32,
                            wz_grid[u] as f32,
                            y0 + heights[u] + 0.5,
                            wx_grid[v] as f32,
                            wz_grid[v] as f32,
                            y0 + heights[v] + 0.5,
                            y0,
                        );
                    }
                }
            }
        }
    }

    // 2. Arched Stone Portals (skip overlap-suppressed interior joins or portals outside table/plate)
    for po in &all_portals {
        if po.suppressed_by_overlap {
            continue;
        }
        let (pwx, pwz) = mountain_local_to_world(po.at.lx, po.at.lz, Some(m));
        if clip_to_support && !is_point_in_any_polygon(pwx, pwz, &poly_offsets, &poly_coords) {
            continue;
        }
        let dwx = po.dir_xz[0] * cos_f - po.dir_xz[1] * sin_f;
        let dwz = po.dir_xz[0] * sin_f + po.dir_xz[1] * cos_f;
        crate::tunnels_3d::push_arched_tunnel_portal_3d(
            verts,
            idxs,
            [pwx as f32, y0 + po.at.rel_elev as f32, pwz as f32],
            [dwx as f32, dwz as f32],
            (y0 + po.at.rel_elev as f32).max(0.0),
            &po.prof,
        );
    }

    // 3. Vaulted Interior Tunnel Linings along all runs (clipped cleanly inside table/plate)
    for tr in &tunnel_runs {
        if tr.pts.len() < 2 {
            continue;
        }
        let mut pts = tr.pts.clone();
        if tr.start_portal.as_ref().map_or(false, |sp| sp.suppressed_by_overlap) {
            let p0 = pts[0];
            let d = tr.start_portal.as_ref().unwrap().dir_xz;
            pts.insert(
                0,
                crate::terrain_tools::LocalTunnelPt {
                    lx: p0.lx - d[0] * 35.0,
                    lz: p0.lz - d[1] * 35.0,
                    rel_elev: p0.rel_elev,
                    norm_r: p0.norm_r,
                },
            );
        }
        if tr.end_portal.as_ref().map_or(false, |ep| ep.suppressed_by_overlap) {
            let pn = *pts.last().unwrap();
            let d = tr.end_portal.as_ref().unwrap().dir_xz;
            pts.push(crate::terrain_tools::LocalTunnelPt {
                lx: pn.lx - d[0] * 35.0,
                lz: pn.lz - d[1] * 35.0,
                rel_elev: pn.rel_elev,
                norm_r: pn.norm_r,
            });
        }
        let mut path_xyz = Vec::with_capacity(pts.len() * 3);
        for p in &pts {
            let (wx, wz) = mountain_local_to_world(p.lx, p.lz, Some(m));
            if !clip_to_support || is_point_in_any_polygon(wx, wz, &poly_offsets, &poly_coords) {
                path_xyz.push(wx as f32);
                path_xyz.push(y0 + p.rel_elev as f32);
                path_xyz.push(wz as f32);
            } else if path_xyz.len() >= 6 {
                crate::tunnels_3d::push_tunnel_bore_3d(verts, idxs, &path_xyz, &tr.prof);
                path_xyz.clear();
            } else {
                path_xyz.clear();
            }
        }
        if path_xyz.len() >= 6 {
            crate::tunnels_3d::push_tunnel_bore_3d(verts, idxs, &path_xyz, &tr.prof);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_diorama_skirt_quad(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    x0: f32,
    z0: f32,
    top0: f32,
    x1: f32,
    z1: f32,
    top1: f32,
    y_bot: f32,
) {
    let dx = x1 - x0;
    let dz = z1 - z0;
    let len = dx.hypot(dz);
    if len < 0.05 {
        return;
    }
    let nx = dz / len;
    let nz = -dx / len;
    let col = [0.30, 0.27, 0.24];
    let base = verts.len() as u32;
    verts.push(Vertex3D::new([x0, y_bot, z0], [nx, 0.0, nz], col, MAT_TERRAIN));
    verts.push(Vertex3D::new([x1, y_bot, z1], [nx, 0.0, nz], col, MAT_TERRAIN));
    verts.push(Vertex3D::new([x1, top1.max(y_bot), z1], [nx, 0.0, nz], col, MAT_TERRAIN));
    verts.push(Vertex3D::new([x0, top0.max(y_bot), z0], [nx, 0.0, nz], col, MAT_TERRAIN));
    // Emit double-sided skirt so diorama cut-away edge looks solid from either side
    idxs.extend_from_slice(&[
        base,
        base + 1,
        base + 2,
        base,
        base + 2,
        base + 3,
        base,
        base + 2,
        base + 1,
        base,
        base + 3,
        base + 2,
    ]);
}

