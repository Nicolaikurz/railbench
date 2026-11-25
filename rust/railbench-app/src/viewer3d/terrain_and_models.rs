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
fn push_diorama_skirt_quad(
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

#[allow(clippy::too_many_arguments)]
pub(super) fn push_ground_terrain_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    m: &Model3D,
    y0: f32,
    layout: &Layout,
    layer_info: &std::collections::HashMap<&str, (f64, bool)>,
    boards: &[BaseboardConfig],
    corridors: &[CorridorChain],
) {
    let scale = m.scale.unwrap_or(1.0).max(0.05);
    let width = (m.width * scale).max(60.0);
    let depth = (m.depth * scale).max(60.0);
    let ttype = m.ground_terrain_type.as_deref().unwrap_or("hill");
    let is_water = ttype == "water";
    let default_h: f64 = match ttype {
        "water" => 6.0,
        "sand" => 12.0,
        "dirt" => 14.0,
        _ => 26.0,
    };
    let max_height = (if m.height > 0.0 { m.height } else { default_h }).max(2.0) * scale;
    let mx = m.transform.x;
    let mz = m.transform.y;
    let rot_rad = m.transform.rotation.to_radians();
    let cos_m = rot_rad.cos();
    let sin_m = rot_rad.sin();

    let segs_x = 44usize;
    let segs_z = 44usize;
    let cols = segs_x + 1;
    let rows = segs_z + 1;
    let has_path = m.mountain_path.as_ref().map_or(false, |p| !p.is_empty());

    let mut r_dist_grid = Vec::with_capacity(cols * rows);
    for iz in 0..rows {
        let lz = (iz as f64 / segs_z as f64 - 0.5) * depth;
        let nz = lz / (depth * 0.5);
        for ix in 0..cols {
            let lx = (ix as f64 / segs_x as f64 - 0.5) * width;
            let nx = lx / (width * 0.5);
            let nr = if has_path {
                evaluate_mountain_local(lx, lz, m).0
            } else {
                nx.hypot(nz)
            };
            r_dist_grid.push(nr as f32);
        }
    }

    // Support polygon from tables + active layer wood plates
    let support_polys = collect_terrain_support_polygons(layout, m.layer_id.as_deref(), boards);
    let clip_to_support = !support_polys.is_empty();
    let (poly_offsets, poly_coords) = crate::terrain_tools::pack_support_polygons(&support_polys);

    // Buildings to flatten ground underneath
    let mut buildings: Vec<f32> = Vec::new();
    if let Some(models) = &layout.models {
        for other in models {
            if other.id == m.id {
                continue;
            }
            if matches!(other.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain")) {
                continue;
            }
            let lid = other.layer_id.as_deref().unwrap_or("layer-1");
            if !layer_info.get(lid).map_or(true, |x| x.1) {
                continue;
            }
            let bscale = other.scale.unwrap_or(1.0);
            let brot = (-other.transform.rotation).to_radians();
            buildings.push(other.transform.x as f32);
            buildings.push(other.transform.y as f32);
            buildings.push(brot.cos() as f32);
            buildings.push(brot.sin() as f32);
            buildings.push((other.width * bscale * 0.5 + 10.0) as f32);
            buildings.push((other.depth * bscale * 0.5 + 10.0) as f32);
        }
    }

    // Corridors metadata + flattened points
    let mut corridor_meta: Vec<f32> = Vec::with_capacity(corridors.len() * 8);
    let mut corridor_pts: Vec<f32> = Vec::new();
    for br in corridors {
        if br.pts_xze.is_empty() {
            continue;
        }
        let (mut min_x, mut max_x, mut min_z, mut max_z) =
            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
        let pad = br.half_w + 48.0;
        let pt_start = (corridor_pts.len() / 3) as f32;
        for p in &br.pts_xze {
            min_x = min_x.min(p[0] - pad);
            max_x = max_x.max(p[0] + pad);
            min_z = min_z.min(p[1] - pad);
            max_z = max_z.max(p[1] + pad);
            corridor_pts.push(p[0]);
            corridor_pts.push(p[1]);
            corridor_pts.push(p[2]);
        }
        corridor_meta.extend_from_slice(&[
            min_x,
            max_x,
            min_z,
            max_z,
            br.half_w,
            if br.is_turntable { 1.0 } else { 0.0 },
            pt_start,
            br.pts_xze.len() as f32,
        ]);
    }

    let grid = carve_ground_terrain_grid(
        segs_x,
        segs_z,
        width,
        depth,
        max_height,
        y0 as f64,
        mx,
        mz,
        cos_m,
        sin_m,
        46.0,
        is_water,
        &r_dist_grid,
        &poly_offsets,
        &poly_coords,
        &buildings,
        &corridor_meta,
        &corridor_pts,
    );

    let mut wx_g = vec![0.0_f64; cols * rows];
    let mut wz_g = vec![0.0_f64; cols * rows];
    let mut gh_g = vec![0.0_f32; cols * rows];
    let mut in_ok = vec![false; cols * rows];
    let mut edge_clamped = vec![false; cols * rows];
    let mut is_w_g = vec![false; cols * rows];
    let mut flat_g = vec![0.0_f32; cols * rows];

    for iz in 0..rows {
        let lz = (iz as f64 / segs_z as f64 - 0.5) * depth;
        for ix in 0..cols {
            let lx = (ix as f64 / segs_x as f64 - 0.5) * width;
            let v_idx = iz * cols + ix;
            wx_g[v_idx] = mx + (lx * cos_m - lz * sin_m);
            wz_g[v_idx] = mz + (lx * sin_m + lz * cos_m);
            gh_g[v_idx] = grid[v_idx * 4];
            in_ok[v_idx] = grid[v_idx * 4 + 1] > 0.5;
            is_w_g[v_idx] = grid[v_idx * 4 + 2] > 0.5;
            flat_g[v_idx] = grid[v_idx * 4 + 3];
        }
    }

    if clip_to_support {
        for iz in 0..rows {
            for ix in 0..cols {
                let v_idx = iz * cols + ix;
                if in_ok[v_idx] {
                    continue;
                }
                let mut sum_h = 0.0_f32;
                let mut sum_flat = 0.0_f32;
                let mut w_votes = 0usize;
                let mut nb_cnt = 0usize;
                for dz in -1isize..=1 {
                    for dx in -1isize..=1 {
                        if dx == 0 && dz == 0 {
                            continue;
                        }
                        let nxi = ix as isize + dx;
                        let nzi = iz as isize + dz;
                        if nxi >= 0 && nxi < cols as isize && nzi >= 0 && nzi < rows as isize {
                            let ni = nzi as usize * cols + nxi as usize;
                            if in_ok[ni] {
                                sum_h += gh_g[ni];
                                sum_flat += flat_g[ni];
                                if is_w_g[ni] {
                                    w_votes += 1;
                                }
                                nb_cnt += 1;
                            }
                        }
                    }
                }
                if nb_cnt > 0 {
                    let clamped = crate::terrain_tools::clamp_point_to_support_polygons(
                        Vec2 { x: wx_g[v_idx], y: wz_g[v_idx] },
                        &support_polys,
                        &poly_offsets,
                        &poly_coords,
                    );
                    wx_g[v_idx] = clamped.x;
                    wz_g[v_idx] = clamped.y;
                    gh_g[v_idx] = sum_h / nb_cnt as f32;
                    flat_g[v_idx] = sum_flat / nb_cnt as f32;
                    is_w_g[v_idx] = w_votes > 0;
                    edge_clamped[v_idx] = true;
                }
            }
        }
    }

    let step_x = (width / segs_x as f64) as f32;
    let step_z = (depth / segs_z as f64) as f32;
    let base_v = verts.len() as u32;

    for iz in 0..rows {
        let lz = (iz as f64 / segs_z as f64 - 0.5) * depth;
        let nz = lz / (depth * 0.5);
        for ix in 0..cols {
            let lx = (ix as f64 / segs_x as f64 - 0.5) * width;
            let nx = lx / (width * 0.5);
            let n_col = (mountain_noise(nx * 1.4, nz * 1.4) * 0.04) as f32;
            let v_idx = iz * cols + ix;
            let gh = gh_g[v_idx];
            let is_w = is_w_g[v_idx];
            let flat = flat_g[v_idx];

            let hl = gh_g[iz * cols + ix.saturating_sub(1)];
            let hr = gh_g[iz * cols + (ix + 1).min(segs_x)];
            let hd = gh_g[iz.saturating_sub(1) * cols + ix];
            let hu = gh_g[(iz + 1).min(segs_z) * cols + ix];
            let norm = normalize3([(hl - hr) / (2.0 * step_x), 1.0, (hd - hu) / (2.0 * step_z)]);
            let (rnx, rnz) = rot_xz(norm[0], norm[2], rot_rad as f32);

            let h_ratio = (gh / max_height.max(1.0) as f32).clamp(0.0, 1.0);
            let col = match ttype {
                "sand" => [
                    (0.84 + h_ratio * 0.06 + n_col - flat * 0.04).clamp(0.0, 1.0),
                    (0.74 + h_ratio * 0.05 + n_col - flat * 0.04).clamp(0.0, 1.0),
                    (0.51 + h_ratio * 0.03 + n_col).clamp(0.0, 1.0),
                ],
                "dirt" => [
                    (0.44 + h_ratio * 0.08 + n_col).clamp(0.0, 1.0),
                    (0.32 + h_ratio * 0.06 + n_col).clamp(0.0, 1.0),
                    (0.21 + h_ratio * 0.04 + n_col).clamp(0.0, 1.0),
                ],
                "water" => {
                    if !is_w || flat > 0.35 || h_ratio > 0.25 {
                        [0.74 + n_col, 0.66 + n_col, 0.48 + n_col]
                    } else {
                        [0.14 + n_col * 0.5, 0.32 + n_col * 0.5, 0.42 + n_col * 0.5]
                    }
                }
                _ => [
                    (0.24 + h_ratio * 0.10 + n_col).clamp(0.0, 1.0),
                    (0.42 + h_ratio * 0.12 + n_col).clamp(0.0, 1.0),
                    (0.14 + h_ratio * 0.04 + n_col).clamp(0.0, 1.0),
                ],
            };

            verts.push(Vertex3D::new(
                [wx_g[v_idx] as f32, y0 + gh + 0.3, wz_g[v_idx] as f32],
                [rnx, norm[1], rnz],
                col,
                MAT_TERRAIN,
            ));
        }
    }

    let valid_gnd_tri = |ia: usize, ib: usize, ic: usize| -> bool {
        if !(in_ok[ia] || edge_clamped[ia])
            || !(in_ok[ib] || edge_clamped[ib])
            || !(in_ok[ic] || edge_clamped[ic])
        {
            return false;
        }
        if !clip_to_support {
            return true;
        }
        let tri_cx = (wx_g[ia] + wx_g[ib] + wx_g[ic]) / 3.0;
        let tri_cz = (wz_g[ia] + wz_g[ib] + wz_g[ic]) / 3.0;
        is_point_in_any_polygon(tri_cx, tri_cz, &poly_offsets, &poly_coords)
    };

    for iz in 0..segs_z {
        for ix in 0..segs_x {
            let i_a = iz * cols + ix;
            let i_b = i_a + 1;
            let i_c = (iz + 1) * cols + ix;
            let i_d = i_c + 1;
            let vis_h = |vi: usize| gh_g[vi] > 0.08;
            if (vis_h(i_a) || vis_h(i_b) || vis_h(i_c)) && valid_gnd_tri(i_a, i_c, i_b) {
                idxs.extend_from_slice(&[base_v + i_a as u32, base_v + i_c as u32, base_v + i_b as u32]);
                if !is_water {
                    for &(u, v) in &[(i_a, i_c), (i_c, i_b), (i_b, i_a)] {
                        if edge_clamped[u] && edge_clamped[v] && gh_g[u].max(gh_g[v]) > 0.5 {
                            push_diorama_skirt_quad(
                                verts,
                                idxs,
                                wx_g[u] as f32,
                                wz_g[u] as f32,
                                y0 + gh_g[u] + 0.3,
                                wx_g[v] as f32,
                                wz_g[v] as f32,
                                y0 + gh_g[v] + 0.3,
                                y0,
                            );
                        }
                    }
                }
            }
            if (vis_h(i_b) || vis_h(i_c) || vis_h(i_d)) && valid_gnd_tri(i_b, i_c, i_d) {
                idxs.extend_from_slice(&[base_v + i_b as u32, base_v + i_c as u32, base_v + i_d as u32]);
                if !is_water {
                    for &(u, v) in &[(i_b, i_c), (i_c, i_d), (i_d, i_b)] {
                        if edge_clamped[u] && edge_clamped[v] && gh_g[u].max(gh_g[v]) > 0.5 {
                            push_diorama_skirt_quad(
                                verts,
                                idxs,
                                wx_g[u] as f32,
                                wz_g[u] as f32,
                                y0 + gh_g[u] + 0.3,
                                wx_g[v] as f32,
                                wz_g[v] as f32,
                                y0 + gh_g[v] + 0.3,
                                y0,
                            );
                        }
                    }
                }
            }
        }
    }

    // Reflective cyan water surface over water basin cells (strictly clipped to table/plate)
    if is_water {
        let water_base_v = verts.len() as u32;
        let water_y = y0 + 1.85;
        let water_col = [0.06, 0.56, 0.84];
        for v_idx in 0..(cols * rows) {
            verts.push(Vertex3D::new(
                [wx_g[v_idx] as f32, water_y, wz_g[v_idx] as f32],
                [0.0, 1.0, 0.0],
                water_col,
                MAT_WATER,
            ));
        }
        let is_wat = |vi: usize| (in_ok[vi] || edge_clamped[vi]) && is_w_g[vi];
        for iz in 0..segs_z {
            for ix in 0..segs_x {
                let i_a = iz * cols + ix;
                let i_b = i_a + 1;
                let i_c = (iz + 1) * cols + ix;
                let i_d = i_c + 1;
                if is_wat(i_a) && is_wat(i_b) && is_wat(i_c) && valid_gnd_tri(i_a, i_c, i_b) {
                    idxs.extend_from_slice(&[
                        water_base_v + i_a as u32,
                        water_base_v + i_c as u32,
                        water_base_v + i_b as u32,
                    ]);
                }
                if is_wat(i_b) && is_wat(i_c) && is_wat(i_d) && valid_gnd_tri(i_b, i_c, i_d) {
                    idxs.extend_from_slice(&[
                        water_base_v + i_b as u32,
                        water_base_v + i_c as u32,
                        water_base_v + i_d as u32,
                    ]);
                }
            }
        }
    }
}

pub(super) fn push_model_3d(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, m: &Model3D, y0: f32) {
    let scale = m.scale.unwrap_or(1.0) as f32;
    let cx = m.transform.x as f32;
    let cz = m.transform.y as f32;
    let rot_rad = (m.transform.rotation as f32).to_radians();

    // Imported custom .glb / .gltf mesh
    if m.file_data.is_some() && matches!(m.model_type.as_str(), "glb" | "gltf") {
        if let Some(parsed) = crate::model_tools::decode_custom_model_mesh(m) {
            let cx_mesh = (parsed.min[0] + parsed.max[0]) * 0.5;
            let cz_mesh = (parsed.min[2] + parsed.max[2]) * 0.5;
            let base_y = parsed.min[1];
            for tri in &parsed.triangles {
                let base_v = verts.len() as u32;
                for k in 0..3 {
                    let lx = (tri.positions[k][0] - cx_mesh) * scale;
                    let ly = (tri.positions[k][1] - base_y) * scale;
                    let lz = (tri.positions[k][2] - cz_mesh) * scale;
                    let (rx, rz) = rot_xz(lx, lz, rot_rad);
                    let (nx, nz) = rot_xz(tri.normals[k][0], tri.normals[k][2], rot_rad);
                    verts.push(Vertex3D::new(
                        [cx + rx, y0 + ly, cz + rz],
                        normalize3([nx, tri.normals[k][1], nz]),
                        tri.color,
                        [0.68, 0.08, 0.0],
                    ));
                }
                idxs.extend_from_slice(&[base_v, base_v + 1, base_v + 2]);
            }
            return;
        }
    }

    crate::models3d_mesh::push_builtin_model_3d(verts, idxs, m, y0);
}

// ============================================================================
//  3D Geometry Primitives (Trapezoidal Ballast, Box Segments, Rotated Boxes)
// ============================================================================

pub(crate) fn rot_xz(x: f32, z: f32, rad: f32) -> (f32, f32) {
    let c = rad.cos();
    let s = rad.sin();
    (x * c - z * s, x * s + z * c)
}

#[inline]
fn heightfield_vertex_color(h: f32, ny: f32, mat: u8, wx: f64, wz: f64) -> [f32; 3] {
    use railbench_core::terrain::{
        ridged_fbm_noise, BIOME_DIRT, BIOME_FOREST, BIOME_GRASS, BIOME_ROCK, BIOME_SAND,
        BIOME_SNOW, BIOME_WATER,
    };
    let nv = (ridged_fbm_noise(wx / 140.0, wz / 140.0) as f32 - 0.5) * 0.08;
    match mat {
        BIOME_GRASS => [
            (0.24 + nv).clamp(0.0, 1.0),
            (0.45 + nv * 1.1).clamp(0.0, 1.0),
            (0.14 + nv * 0.5).clamp(0.0, 1.0),
        ],
        BIOME_FOREST => [
            (0.13 + nv * 0.6).clamp(0.0, 1.0),
            (0.29 + nv * 0.8).clamp(0.0, 1.0),
            (0.12 + nv * 0.5).clamp(0.0, 1.0),
        ],
        BIOME_ROCK => [
            (0.39 + nv).clamp(0.0, 1.0),
            (0.38 + nv).clamp(0.0, 1.0),
            (0.36 + nv).clamp(0.0, 1.0),
        ],
        BIOME_SNOW => [
            (0.92 + nv * 0.4).clamp(0.0, 1.0),
            (0.95 + nv * 0.4).clamp(0.0, 1.0),
            (0.99 + nv * 0.2).clamp(0.0, 1.0),
        ],
        BIOME_SAND => [
            (0.78 + nv).clamp(0.0, 1.0),
            (0.69 + nv).clamp(0.0, 1.0),
            (0.47 + nv * 0.8).clamp(0.0, 1.0),
        ],
        BIOME_DIRT => [
            (0.42 + nv).clamp(0.0, 1.0),
            (0.30 + nv * 0.8).clamp(0.0, 1.0),
            (0.19 + nv * 0.6).clamp(0.0, 1.0),
        ],
        BIOME_WATER => [0.40, 0.37, 0.28],
        _ => {
            // BIOME_AUTO: natural elevation + slope-aware alpine blending
            let grass = [0.24 + nv, 0.44 + nv, 0.14 + nv * 0.5];
            let forest = [0.15 + nv * 0.7, 0.31 + nv * 0.8, 0.13 + nv * 0.5];
            let rock = [0.38 + nv, 0.37 + nv, 0.35 + nv];
            let snow = [0.94, 0.97, 1.00];
            let mix3 = |a: [f32; 3], b: [f32; 3], t: f32| -> [f32; 3] {
                let tc = t.clamp(0.0, 1.0);
                [
                    (a[0] + (b[0] - a[0]) * tc).clamp(0.0, 1.0),
                    (a[1] + (b[1] - a[1]) * tc).clamp(0.0, 1.0),
                    (a[2] + (b[2] - a[2]) * tc).clamp(0.0, 1.0),
                ]
            };
            if h < -1.0 {
                [0.44, 0.40, 0.31]
            } else if h >= 210.0 {
                let st = smoothstep(200.0, 260.0, h as f64) as f32;
                let base_r = if ny < 0.76 { rock } else { forest };
                mix3(mix3(base_r, rock, 0.5), snow, st)
            } else if ny < 0.78 {
                let cliff_t = smoothstep(0.62, 0.80, ny as f64) as f32;
                mix3(rock, forest, cliff_t)
            } else if h > 95.0 {
                let rt = smoothstep(95.0, 185.0, h as f64) as f32;
                mix3(forest, rock, rt * 0.75)
            } else {
                let ft = smoothstep(18.0, 95.0, h as f64) as f32;
                mix3(grass, forest, ft)
            }
        }
    }
}

/// Builds the unified per-board continuous 3D heightfield mesh, strictly clipped to enabled
/// baseboards/plates with diorama edge skirts, building/track corridor carving, tunnel bores/portals,
/// and water surface quads.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_heightfield_terrain_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    hf: &railbench_core::types::TerrainHeightfield,
    layer_id: Option<&str>,
    y0: f32,
    layout: &Layout,
    boards: &[BaseboardConfig],
    chains: &[CorridorChain],
    layer_info: &std::collections::HashMap<&str, (f64, bool)>,
) {
    use railbench_core::terrain::{sample_heightfield_bilinear, BIOME_WATER};

    let cols = hf.cols;
    let rows = hf.rows;
    if cols < 2
        || rows < 2
        || hf.heights.len() != cols * rows
        || hf.materials.len() != cols * rows
    {
        return;
    }
    let span_x = (hf.max_x - hf.min_x).max(1.0);
    let span_z = (hf.max_z - hf.min_z).max(1.0);
    let dx = span_x / (cols - 1) as f64;
    let dz = span_z / (rows - 1) as f64;

    let support_polys = collect_terrain_support_polygons(layout, layer_id, boards);
    let clip_to_support = !support_polys.is_empty();
    let (poly_offsets, poly_coords) = crate::terrain_tools::pack_support_polygons(&support_polys);

    let total = cols * rows;
    let mut raw_h = hf.heights.as_ref().clone();
    let mut mats = hf.materials.as_ref().clone();
    let mut eff_h = raw_h.clone();
    let mut wx_grid = vec![0.0_f64; total];
    let mut wz_grid = vec![0.0_f64; total];
    let mut inside_sup = vec![true; total];
    let mut edge_clamped = vec![false; total];

    for iz in 0..rows {
        let wz = hf.min_z + iz as f64 * dz;
        for ix in 0..cols {
            let v_idx = iz * cols + ix;
            let wx = hf.min_x + ix as f64 * dx;
            wx_grid[v_idx] = wx;
            wz_grid[v_idx] = wz;
            if clip_to_support && !is_point_in_any_polygon(wx, wz, &poly_offsets, &poly_coords) {
                inside_sup[v_idx] = false;
                raw_h[v_idx] = 0.0;
                eff_h[v_idx] = 0.0;
                mats[v_idx] = 0;
            }
        }
    }

    if clip_to_support {
        for iz in 0..rows {
            for ix in 0..cols {
                let v_idx = iz * cols + ix;
                if inside_sup[v_idx] {
                    continue;
                }
                let mut has_in_nb = false;
                'nb_hf: for oz in -1isize..=1 {
                    for ox in -1isize..=1 {
                        if ox == 0 && oz == 0 {
                            continue;
                        }
                        let nxi = ix as isize + ox;
                        let nzi = iz as isize + oz;
                        if nxi >= 0 && nxi < cols as isize && nzi >= 0 && nzi < rows as isize {
                            if inside_sup[nzi as usize * cols + nxi as usize] {
                                has_in_nb = true;
                                break 'nb_hf;
                            }
                        }
                    }
                }
                if has_in_nb {
                    let clamped = crate::terrain_tools::clamp_point_to_support_polygons(
                        Vec2 {
                            x: wx_grid[v_idx],
                            y: wz_grid[v_idx],
                        },
                        &support_polys,
                        &poly_offsets,
                        &poly_coords,
                    );
                    wx_grid[v_idx] = clamped.x;
                    wz_grid[v_idx] = clamped.y;
                    let ch = sample_heightfield_bilinear(hf, clamped.x, clamped.y);
                    raw_h[v_idx] = ch;
                    eff_h[v_idx] = ch;
                    edge_clamped[v_idx] = true;
                }
            }
        }
    }

    // 1. Building footprint pads
    if let Some(models) = &layout.models {
        for other in models {
            if matches!(
                other.builtin_id.as_deref(),
                Some("mountain") | Some("ground-terrain")
            ) {
                continue;
            }
            let lid = other.layer_id.as_deref().unwrap_or("layer-1");
            if let Some(target_lid) = layer_id {
                if lid != target_lid {
                    continue;
                }
            }
            let (lh, vis) = layer_info.get(lid).copied().unwrap_or((0.0, true));
            if !vis {
                continue;
            }
            let by0 = ((lh + other.elevation.unwrap_or(0.0)) as f32 - y0).max(0.0);
            let bscale = other.scale.unwrap_or(1.0);
            let bhw = (other.width * bscale * 0.5 + 10.0) as f32;
            let bhd = (other.depth * bscale * 0.5 + 10.0) as f32;
            let brot = (-other.transform.rotation).to_radians();
            let cos_b = brot.cos();
            let sin_b = brot.sin();
            let reach = bhw.max(bhd) as f64 + 42.0;

            let ix0 = (((other.transform.x - reach - hf.min_x) / dx).floor() as isize)
                .clamp(0, cols as isize - 1) as usize;
            let ix1 = (((other.transform.x + reach - hf.min_x) / dx).ceil() as isize)
                .clamp(0, cols as isize - 1) as usize;
            let iz0 = (((other.transform.y - reach - hf.min_z) / dz).floor() as isize)
                .clamp(0, rows as isize - 1) as usize;
            let iz1 = (((other.transform.y + reach - hf.min_z) / dz).ceil() as isize)
                .clamp(0, rows as isize - 1) as usize;

            for iz in iz0..=iz1 {
                for ix in ix0..=ix1 {
                    let v_idx = iz * cols + ix;
                    if !inside_sup[v_idx] && !edge_clamped[v_idx] {
                        continue;
                    }
                    if eff_h[v_idx].abs() < 0.3 && mats[v_idx] == 0 {
                        continue;
                    }
                    let ddx = wx_grid[v_idx] - other.transform.x;
                    let ddz = wz_grid[v_idx] - other.transform.y;
                    let lx = (ddx * cos_b - ddz * sin_b).abs() as f32 - bhw;
                    let lz = (ddx * sin_b + ddz * cos_b).abs() as f32 - bhd;
                    let sdf = lx.max(lz).max(0.0);
                    if sdf < 38.0 {
                        let w_flat =
                            1.0 - smoothstep(0.0, 38.0, sdf as f64) as f32;
                        eff_h[v_idx] = eff_h[v_idx] + (by0 - eff_h[v_idx]) * w_flat;
                    }
                }
            }
        }
    }

    // 2. Track/road corridor carving + buried tunnel detection
    struct HfPortal {
        pos: [f32; 3],
        dir_xz: [f32; 2],
        prof: TunnelProfile,
    }
    let mut portals: Vec<HfPortal> = Vec::new();
    let mut tunnel_bores: Vec<(Vec<f32>, TunnelProfile)> = Vec::new();

    for ch in chains {
        if ch.pts_xze.len() < 2 {
            continue;
        }
        let prof = create_tunnel_profile(ch.is_road, ch.has_catenary, ch.gauge, 1, 0.0);
        let inner = prof.inner_width * 0.5 + 4.0;
        let outer = inner + 44.0;

        // Track corridor carving (shallow mounds carved flat; high overburden keeps tunnel roof)
        let mut pmin_x = f64::INFINITY;
        let mut pmax_x = f64::NEG_INFINITY;
        let mut pmin_z = f64::INFINITY;
        let mut pmax_z = f64::NEG_INFINITY;
        let mut any_near_layer = false;
        for pt in &ch.pts_xze {
            let rel_e = pt[2] - y0;
            if rel_e < -25.0 || rel_e > crate::terrain_tools::MAX_HEIGHT_MM {
                continue;
            }
            any_near_layer = true;
            pmin_x = pmin_x.min(pt[0] as f64 - outer as f64);
            pmax_x = pmax_x.max(pt[0] as f64 + outer as f64);
            pmin_z = pmin_z.min(pt[1] as f64 - outer as f64);
            pmax_z = pmax_z.max(pt[1] as f64 + outer as f64);
        }
        if !any_near_layer {
            continue;
        }
        let ix0 = (((pmin_x - hf.min_x) / dx).floor() as isize).clamp(0, cols as isize - 1) as usize;
        let ix1 = (((pmax_x - hf.min_x) / dx).ceil() as isize).clamp(0, cols as isize - 1) as usize;
        let iz0 = (((pmin_z - hf.min_z) / dz).floor() as isize).clamp(0, rows as isize - 1) as usize;
        let iz1 = (((pmax_z - hf.min_z) / dz).ceil() as isize).clamp(0, rows as isize - 1) as usize;

        for iz in iz0..=iz1 {
            for ix in ix0..=ix1 {
                let v_idx = iz * cols + ix;
                if !inside_sup[v_idx] && !edge_clamped[v_idx] {
                    continue;
                }
                let vx = wx_grid[v_idx] as f32;
                let vz = wz_grid[v_idx] as f32;
                let mut best_d_sq = f32::INFINITY;
                let mut best_elev = 0.0_f32;
                for pt in &ch.pts_xze {
                    let rel_e = pt[2] - y0;
                    if rel_e < -25.0 {
                        continue;
                    }
                    let d_sq = (vx - pt[0]).powi(2) + (vz - pt[1]).powi(2);
                    if d_sq < best_d_sq {
                        best_d_sq = d_sq;
                        best_elev = rel_e;
                    }
                }
                if best_d_sq <= outer * outer {
                    let dist = best_d_sq.sqrt();
                    let cur_h = raw_h[v_idx];
                    if cur_h >= best_elev + 42.0 {
                        if dist <= inner + 14.0 {
                            let target = best_elev + prof.total_h + 10.0;
                            let blend = 1.0
                                - smoothstep(inner as f64, (inner + 14.0) as f64, dist as f64)
                                    as f32;
                            if eff_h[v_idx] < target {
                                eff_h[v_idx] += (target - eff_h[v_idx]) * blend;
                            }
                        }
                    } else if cur_h > best_elev - 1.5 {
                        let target = (best_elev - 1.2).max(0.0);
                        let blend = 1.0
                            - smoothstep(inner as f64, outer as f64, dist as f64) as f32;
                        if eff_h[v_idx] > target {
                            eff_h[v_idx] += (target - eff_h[v_idx]) * blend;
                        }
                    }
                }
            }
        }

        // Detect underground runs in this heightfield and emit tunnel portals + bores
        let mut run_pts: Vec<[f32; 3]> = Vec::new();
        let n_pts = ch.pts_xze.len();
        let flush_run = |run: &mut Vec<[f32; 3]>,
                         portals: &mut Vec<HfPortal>,
                         tunnel_bores: &mut Vec<(Vec<f32>, TunnelProfile)>| {
            if run.len() >= 2 {
                let p0 = run[0];
                let p1 = run[1];
                let len0 = (p1[0] - p0[0]).hypot(p1[1] - p0[1]);
                if len0 > 0.5 {
                    portals.push(HfPortal {
                        pos: [p0[0], p0[2].max(y0), p0[1]],
                        dir_xz: [(p0[0] - p1[0]) / len0, (p0[1] - p1[1]) / len0],
                        prof: prof.clone(),
                    });
                }
                let pe = run[run.len() - 1];
                let pp = run[run.len() - 2];
                let len1 = (pe[0] - pp[0]).hypot(pe[1] - pp[1]);
                if len1 > 0.5 {
                    portals.push(HfPortal {
                        pos: [pe[0], pe[2].max(y0), pe[1]],
                        dir_xz: [(pe[0] - pp[0]) / len1, (pe[1] - pp[1]) / len1],
                        prof: prof.clone(),
                    });
                }
                let mut xyz = Vec::with_capacity(run.len() * 3);
                for q in run.iter() {
                    xyz.push(q[0]);
                    xyz.push(q[2]);
                    xyz.push(q[1]);
                }
                tunnel_bores.push((xyz, prof.clone()));
            }
            run.clear();
        };
        for (pi, &pt) in ch.pts_xze.iter().enumerate() {
            let rel_e = pt[2] - y0;
            let surf = sample_heightfield_bilinear(hf, pt[0] as f64, pt[1] as f64);
            let inside_sup_pt =
                !clip_to_support || is_point_in_any_polygon(pt[0] as f64, pt[1] as f64, &poly_offsets, &poly_coords);
            if inside_sup_pt && rel_e >= -25.0 && surf >= rel_e + 42.0 {
                run_pts.push(pt);
            } else {
                flush_run(&mut run_pts, &mut portals, &mut tunnel_bores);
            }
            if pi + 1 == n_pts {
                flush_run(&mut run_pts, &mut portals, &mut tunnel_bores);
            }
        }
    }

    // 3. Emit Heightfield Terrain Mesh
    let step_x = dx as f32;
    let step_z = dz as f32;
    let base_v = verts.len() as u32;

    for iz in 0..rows {
        for ix in 0..cols {
            let v_idx = iz * cols + ix;
            let h_val = eff_h[v_idx];
            let hl = eff_h[iz * cols + ix.saturating_sub(1)];
            let hr = eff_h[iz * cols + (ix + 1).min(cols - 1)];
            let hd = eff_h[iz.saturating_sub(1) * cols + ix];
            let hu = eff_h[(iz + 1).min(rows - 1) * cols + ix];
            let norm = normalize3([(hl - hr) / (2.0 * step_x), 1.0, (hd - hu) / (2.0 * step_z)]);
            let col = heightfield_vertex_color(h_val, norm[1], mats[v_idx], wx_grid[v_idx], wz_grid[v_idx]);
            verts.push(Vertex3D::new(
                [wx_grid[v_idx] as f32, y0 + h_val + 0.5, wz_grid[v_idx] as f32],
                norm,
                col,
                MAT_TERRAIN,
            ));
        }
    }

    let valid_hf_tri = |ia: usize, ib: usize, ic: usize| -> bool {
        if !(inside_sup[ia] || edge_clamped[ia])
            || !(inside_sup[ib] || edge_clamped[ib])
            || !(inside_sup[ic] || edge_clamped[ic])
        {
            return false;
        }
        if !clip_to_support {
            return true;
        }
        let cx = (wx_grid[ia] + wx_grid[ib] + wx_grid[ic]) / 3.0;
        let cz = (wz_grid[ia] + wz_grid[ib] + wz_grid[ic]) / 3.0;
        is_point_in_any_polygon(cx, cz, &poly_offsets, &poly_coords)
    };

    let is_portal_curtain_quad = |i_a: usize, i_b: usize, i_c: usize, i_d: usize| -> bool {
        if portals.is_empty() {
            return false;
        }
        let min_y = y0 + eff_h[i_a].min(eff_h[i_b]).min(eff_h[i_c]).min(eff_h[i_d]);
        let max_y = y0 + eff_h[i_a].max(eff_h[i_b]).max(eff_h[i_c]).max(eff_h[i_d]);
        let pts = [i_a, i_b, i_c, i_d];
        for po in &portals {
            if min_y >= po.pos[1] + po.prof.total_h || max_y <= po.pos[1] - 2.0 {
                continue;
            }
            let (mut min_fwd, mut max_fwd, mut min_lat, mut max_lat) =
                (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
            for &vi in &pts {
                let ddx = wx_grid[vi] as f32 - po.pos[0];
                let ddz = wz_grid[vi] as f32 - po.pos[2];
                let fwd = ddx * po.dir_xz[0] + ddz * po.dir_xz[1];
                let lat = ddx * (-po.dir_xz[1]) + ddz * po.dir_xz[0];
                min_fwd = min_fwd.min(fwd);
                max_fwd = max_fwd.max(fwd);
                min_lat = min_lat.min(lat);
                max_lat = max_lat.max(lat);
            }
            let hw = po.prof.inner_width * 0.5 + 4.0;
            if max_lat >= -hw && min_lat <= hw && max_fwd >= -38.0 && min_fwd <= 46.0 {
                return true;
            }
        }
        false
    };

    for iz in 0..(rows - 1) {
        for ix in 0..(cols - 1) {
            let i_a = iz * cols + ix;
            let i_b = i_a + 1;
            let i_c = (iz + 1) * cols + ix;
            let i_d = i_c + 1;
            let touched = eff_h[i_a].abs() > 0.35
                || eff_h[i_b].abs() > 0.35
                || eff_h[i_c].abs() > 0.35
                || eff_h[i_d].abs() > 0.35
                || mats[i_a] != 0
                || mats[i_b] != 0
                || mats[i_c] != 0
                || mats[i_d] != 0;
            if !touched || is_portal_curtain_quad(i_a, i_b, i_c, i_d) {
                continue;
            }
            if valid_hf_tri(i_a, i_c, i_b) {
                idxs.extend_from_slice(&[base_v + i_a as u32, base_v + i_c as u32, base_v + i_b as u32]);
                for &(u, v) in &[(i_a, i_c), (i_c, i_b), (i_b, i_a)] {
                    if edge_clamped[u] && edge_clamped[v] && eff_h[u].max(eff_h[v]) > 0.5 {
                        push_diorama_skirt_quad(
                            verts,
                            idxs,
                            wx_grid[u] as f32,
                            wz_grid[u] as f32,
                            y0 + eff_h[u] + 0.5,
                            wx_grid[v] as f32,
                            wz_grid[v] as f32,
                            y0 + eff_h[v] + 0.5,
                            y0,
                        );
                    }
                }
            }
            if valid_hf_tri(i_b, i_c, i_d) {
                idxs.extend_from_slice(&[base_v + i_b as u32, base_v + i_c as u32, base_v + i_d as u32]);
                for &(u, v) in &[(i_b, i_c), (i_c, i_d), (i_d, i_b)] {
                    if edge_clamped[u] && edge_clamped[v] && eff_h[u].max(eff_h[v]) > 0.5 {
                        push_diorama_skirt_quad(
                            verts,
                            idxs,
                            wx_grid[u] as f32,
                            wz_grid[u] as f32,
                            y0 + eff_h[u] + 0.5,
                            wx_grid[v] as f32,
                            wz_grid[v] as f32,
                            y0 + eff_h[v] + 0.5,
                            y0,
                        );
                    }
                }
            }
        }
    }

    // 4. Water Surface Quads on painted water cells or carved basins
    let has_water = mats.contains(&BIOME_WATER) || eff_h.iter().any(|&v| v < -1.5);
    if has_water {
        let water_base = verts.len() as u32;
        let water_col = [0.14, 0.48, 0.72];
        for v_idx in 0..total {
            let wy = y0 + if eff_h[v_idx] < -1.5 { -0.8 } else { eff_h[v_idx].max(0.0) + 1.6 };
            verts.push(Vertex3D::new(
                [wx_grid[v_idx] as f32, wy, wz_grid[v_idx] as f32],
                [0.0, 1.0, 0.0],
                water_col,
                MAT_WATER,
            ));
        }
        let is_wat = |vi: usize| {
            (inside_sup[vi] || edge_clamped[vi]) && (mats[vi] == BIOME_WATER || eff_h[vi] < -1.5)
        };
        for iz in 0..(rows - 1) {
            for ix in 0..(cols - 1) {
                let i_a = iz * cols + ix;
                let i_b = i_a + 1;
                let i_c = (iz + 1) * cols + ix;
                let i_d = i_c + 1;
                if is_wat(i_a) && is_wat(i_b) && is_wat(i_c) && valid_hf_tri(i_a, i_c, i_b) {
                    idxs.extend_from_slice(&[
                        water_base + i_a as u32,
                        water_base + i_c as u32,
                        water_base + i_b as u32,
                    ]);
                }
                if is_wat(i_b) && is_wat(i_c) && is_wat(i_d) && valid_hf_tri(i_b, i_c, i_d) {
                    idxs.extend_from_slice(&[
                        water_base + i_b as u32,
                        water_base + i_c as u32,
                        water_base + i_d as u32,
                    ]);
                }
            }
        }
    }

    // 5. Arched Tunnel Portals & Interior Vault Linings
    for po in &portals {
        crate::tunnels_3d::push_arched_tunnel_portal_3d(
            verts,
            idxs,
            po.pos,
            po.dir_xz,
            po.pos[1],
            &po.prof,
        );
    }
    for (xyz, prof) in &tunnel_bores {
        if xyz.len() >= 6 {
            crate::tunnels_3d::push_tunnel_bore_3d(verts, idxs, xyz, prof);
        }
    }
}


