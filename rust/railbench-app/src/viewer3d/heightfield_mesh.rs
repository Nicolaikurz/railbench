use super::*;

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


