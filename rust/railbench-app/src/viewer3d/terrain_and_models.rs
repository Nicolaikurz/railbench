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
    let mut poly_offsets: Vec<u32> = Vec::new();
    let mut poly_coords: Vec<f32> = Vec::new();
    if clip_to_support {
        poly_offsets.push(0);
        for poly in &support_polys {
            for pt in poly {
                poly_coords.push(pt.x as f32);
                poly_coords.push(pt.y as f32);
            }
            poly_offsets.push((poly_coords.len() / 2) as u32);
        }
    }

    let mut heights = vec![0.0_f32; cols * rows];
    let mut inside_support = vec![true; cols * rows];
    let mut norm_r_grid = vec![2.0_f32; cols * rows];

    let hw_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let poly_offsets_ref = &poly_offsets;
    let poly_coords_ref = &poly_coords;

    if hw_threads > 1 && rows * cols >= 512 {
        let rows_per_chunk = (rows + hw_threads - 1) / hw_threads;
        let chunk_elems = rows_per_chunk * cols;
        std::thread::scope(|s| {
            for (
                chunk_idx,
                ((h_chunk, sup_chunk), nr_chunk),
            ) in heights
                .chunks_mut(chunk_elems)
                .zip(inside_support.chunks_mut(chunk_elems))
                .zip(norm_r_grid.chunks_mut(chunk_elems))
                .enumerate()
            {
                let row_start = chunk_idx * rows_per_chunk;
                s.spawn(move || {
                    for (local_r, ((h_row, sup_row), nr_row)) in h_chunk
                        .chunks_mut(cols)
                        .zip(sup_chunk.chunks_mut(cols))
                        .zip(nr_chunk.chunks_mut(cols))
                        .enumerate()
                    {
                        let iz = row_start + local_r;
                        let lz = (iz as f64 / nz as f64 - 0.5) * d as f64;
                        for ix in 0..cols {
                            let lx = (ix as f64 / nx as f64 - 0.5) * w as f64;
                            let wx = cx as f64 + (lx * cos_f - lz * sin_f);
                            let wz = cz as f64 + (lx * sin_f + lz * cos_f);
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
        let norm_r_ref = &norm_r_grid;
        let carve_mtn_row = |iz: usize, h_row: &mut [f32]| {
            let lz = (iz as f32 / nz as f32 - 0.5) * d;
            for (ix, h_cell) in h_row.iter_mut().enumerate() {
                let lx = (ix as f32 / nx as f32 - 0.5) * w;
                let v_idx = iz * cols + ix;
                if !inside_sup_ref[v_idx] {
                    continue;
                }
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
        let lz = (iz as f32 / nz as f32 - 0.5) * d;
        for ix in 0..cols {
            let lx = (ix as f32 / nx as f32 - 0.5) * w;
            let v_idx = iz * cols + ix;
            let mh = heights[v_idx];
            let hl = heights[iz * cols + ix.saturating_sub(1)];
            let hr = heights[iz * cols + (ix + 1).min(nx)];
            let hd = heights[iz.saturating_sub(1) * cols + ix];
            let hu = heights[(iz + 1).min(nz) * cols + ix];
            let normal = normalize3([(hl - hr) / (2.0 * step_x), 1.0, (hd - hu) / (2.0 * step_z)]);
            let ratio = (mh / h.max(1.0)).clamp(0.0, 1.0);
            let col = mountain_biome_color(ratio, m.mountain_style.as_deref());
            let (rx, rz) = rot_xz(lx, lz, rot_rad);
            let (rnx, rnz) = rot_xz(normal[0], normal[2], rot_rad);
            verts.push(Vertex3D::new(
                [cx + rx, y0 + mh + 0.5, cz + rz],
                [rnx, normal[1], rnz],
                col,
                MAT_TERRAIN,
            ));
        }
    }

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
            if (ha > 0.1 || hc > 0.1 || hb > 0.1)
                && inside_support[i_a]
                && inside_support[i_c]
                && inside_support[i_b]
            {
                idxs.extend_from_slice(&[base_v + i_a as u32, base_v + i_c as u32, base_v + i_b as u32]);
            }
            if (hb > 0.1 || hc > 0.1 || hd > 0.1)
                && inside_support[i_b]
                && inside_support[i_c]
                && inside_support[i_d]
            {
                idxs.extend_from_slice(&[base_v + i_b as u32, base_v + i_c as u32, base_v + i_d as u32]);
            }
        }
    }

    // 2. Arched Stone Portals (skip overlap-suppressed interior joins)
    for po in &all_portals {
        if po.suppressed_by_overlap {
            continue;
        }
        let (pwx, pwz) = mountain_local_to_world(po.at.lx, po.at.lz, Some(m));
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

    // 3. Vaulted Interior Tunnel Linings along all runs
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
            path_xyz.push(wx as f32);
            path_xyz.push(y0 + p.rel_elev as f32);
            path_xyz.push(wz as f32);
        }
        crate::tunnels_3d::push_tunnel_bore_3d(verts, idxs, &path_xyz, &tr.prof);
    }
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
    let mut poly_offsets: Vec<u32> = Vec::new();
    let mut poly_coords: Vec<f32> = Vec::new();
    if !support_polys.is_empty() {
        poly_offsets.push(0);
        for poly in &support_polys {
            for pt in poly {
                poly_coords.push(pt.x as f32);
                poly_coords.push(pt.y as f32);
            }
            poly_offsets.push((poly_coords.len() / 2) as u32);
        }
    }

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
            let gh = grid[v_idx * 4];
            let is_w = grid[v_idx * 4 + 2] > 0.5;
            let flat = grid[v_idx * 4 + 3];

            let hl = grid[(iz * cols + ix.saturating_sub(1)) * 4];
            let hr = grid[(iz * cols + (ix + 1).min(segs_x)) * 4];
            let hd = grid[(iz.saturating_sub(1) * cols + ix) * 4];
            let hu = grid[((iz + 1).min(segs_z) * cols + ix) * 4];
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

            let wx = mx + (lx * cos_m - lz * sin_m);
            let wz = mz + (lx * sin_m + lz * cos_m);
            verts.push(Vertex3D::new(
                [wx as f32, y0 + gh + 0.3, wz as f32],
                [rnx, norm[1], rnz],
                col,
                MAT_TERRAIN,
            ));
        }
    }

    for iz in 0..segs_z {
        for ix in 0..segs_x {
            let i_a = iz * cols + ix;
            let i_b = i_a + 1;
            let i_c = (iz + 1) * cols + ix;
            let i_d = i_c + 1;
            let in_poly = |vi: usize| grid[vi * 4 + 1] > 0.5;
            let vis_h = |vi: usize| grid[vi * 4] > 0.08;
            if in_poly(i_a)
                && in_poly(i_b)
                && in_poly(i_c)
                && in_poly(i_d)
                && (vis_h(i_a) || vis_h(i_b) || vis_h(i_c) || vis_h(i_d))
            {
                let a = base_v + i_a as u32;
                let b = base_v + i_b as u32;
                let c = base_v + i_c as u32;
                let d_i = base_v + i_d as u32;
                idxs.extend_from_slice(&[a, c, b, b, c, d_i]);
            }
        }
    }

    // Reflective cyan water surface over water basin cells (spares tracks, roads & buildings)
    if is_water {
        let water_base_v = verts.len() as u32;
        let water_y = y0 + 1.85;
        let water_col = [0.06, 0.56, 0.84];
        for iz in 0..rows {
            let lz = (iz as f64 / segs_z as f64 - 0.5) * depth;
            for ix in 0..cols {
                let lx = (ix as f64 / segs_x as f64 - 0.5) * width;
                let wx = mx + (lx * cos_m - lz * sin_m);
                let wz = mz + (lx * sin_m + lz * cos_m);
                verts.push(Vertex3D::new(
                    [wx as f32, water_y, wz as f32],
                    [0.0, 1.0, 0.0],
                    water_col,
                    MAT_WATER,
                ));
            }
        }
        for iz in 0..segs_z {
            for ix in 0..segs_x {
                let i_a = iz * cols + ix;
                let i_b = i_a + 1;
                let i_c = (iz + 1) * cols + ix;
                let i_d = i_c + 1;
                let is_wat = |vi: usize| grid[vi * 4 + 1] > 0.5 && grid[vi * 4 + 2] > 0.5;
                if is_wat(i_a) && is_wat(i_b) && is_wat(i_c) {
                    idxs.extend_from_slice(&[
                        water_base_v + i_a as u32,
                        water_base_v + i_c as u32,
                        water_base_v + i_b as u32,
                    ]);
                }
                if is_wat(i_b) && is_wat(i_c) && is_wat(i_d) {
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

