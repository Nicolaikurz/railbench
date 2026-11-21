use crate::geometry::{is_point_in_any_polygon, smoothstep};

#[inline]
pub fn mountain_noise(nx: f64, nz: f64) -> f64 {
    let s1 = (nx * 3.1415 + 0.5).sin() * (nz * 3.1415 + 0.3).cos();
    let s2 = (nx * 6.28 + nz * 4.71).sin() * 0.35;
    let s3 = (nx * 9.42 - nz * 7.85).cos() * 0.15;
    s1 + s2 + s3
}

#[inline]
pub fn natural_mountain_height_at(nx: f64, nz: f64, height: f64) -> f64 {
    let r_dist = nx.hypot(nz);
    if r_dist >= 1.0 {
        return 0.0;
    }
    let profile = (r_dist * std::f64::consts::FRAC_PI_2).cos().powf(1.35);
    let n = mountain_noise(nx, nz);
    let ridge = (1.0 - (nx * 1.6 + nz * 0.9).sin().abs()) * 0.22;
    let h = height * profile * (0.85 + 0.22 * n + ridge * (1.0 - r_dist));
    h.max(0.0)
}

/// Evaluates a 2D procedural mountain heightfield on a `(nx_steps + 1) x (nz_steps + 1)` grid.
pub fn evaluate_mountain_heightfield(
    nx_steps: usize,
    nz_steps: usize,
    half_w: f64,
    half_d: f64,
    height: f64,
    strokes: &[f32],
) -> Vec<f32> {
    let cols = nx_steps + 1;
    let rows = nz_steps + 1;
    let mut heights = vec![0.0_f32; cols * rows];
    if half_w <= 0.0 || half_d <= 0.0 || height <= 0.0 {
        return heights;
    }

    let has_strokes = strokes.len() >= 4;
    let inv_nx = if nx_steps > 0 { 1.0 / nx_steps as f64 } else { 0.0 };
    let inv_nz = if nz_steps > 0 { 1.0 / nz_steps as f64 } else { 0.0 };

    let eval_row = |iz: usize, row_out: &mut [f32]| {
        let vz = iz as f64 * inv_nz;
        let lz = (vz * 2.0 - 1.0) * half_d;
        let nz = lz / half_d;

        for (ix, cell) in row_out.iter_mut().enumerate() {
            let vx = ix as f64 * inv_nx;
            let lx = (vx * 2.0 - 1.0) * half_w;
            let nx = lx / half_w;

            let h = if !has_strokes {
                natural_mountain_height_at(nx, nz, height)
            } else {
                let n_nodes = strokes.len() / 4;
                let noise_val = mountain_noise(lx / 220.0, lz / 220.0);
                let mut best = 0.0_f64;
                if n_nodes == 1 {
                    let sx = strokes[0] as f64;
                    let sz = strokes[1] as f64;
                    let sr = (strokes[2] as f64).max(25.0);
                    let sh = (strokes[3] as f64).max(10.0);
                    let nr = (lx - sx).hypot(lz - sz) / sr;
                    if nr < 1.0 {
                        let prof = (nr * std::f64::consts::FRAC_PI_2).cos().powf(1.35);
                        best = (sh * prof * (1.0 + 0.26 * noise_val)).max(0.0);
                    }
                } else {
                    for i in 0..(n_nodes - 1) {
                        let ax = strokes[i * 4] as f64;
                        let az = strokes[i * 4 + 1] as f64;
                        let ar = (strokes[i * 4 + 2] as f64).max(25.0);
                        let ah = (strokes[i * 4 + 3] as f64).max(10.0);

                        let bx = strokes[(i + 1) * 4] as f64;
                        let bz = strokes[(i + 1) * 4 + 1] as f64;
                        let br = (strokes[(i + 1) * 4 + 2] as f64).max(25.0);
                        let bh = (strokes[(i + 1) * 4 + 3] as f64).max(10.0);

                        let abx = bx - ax;
                        let abz = bz - az;
                        let len_sq = abx * abx + abz * abz;
                        let t = if len_sq > 1e-4 {
                            (((lx - ax) * abx + (lz - az) * abz) / len_sq).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let px = ax + abx * t;
                        let pz = az + abz * t;
                        let r = ar + (br - ar) * t;
                        let sh = ah + (bh - ah) * t;
                        let nr = (lx - px).hypot(lz - pz) / r;
                        if nr < 1.0 {
                            let prof = (nr * std::f64::consts::FRAC_PI_2).cos().powf(1.35);
                            let cand = (sh * prof * (1.0 + 0.26 * noise_val)).max(0.0);
                            if cand > best {
                                best = cand;
                            }
                        }
                    }
                }
                best
            };

            *cell = h as f32;
        }
    };

    #[cfg(not(target_arch = "wasm32"))]
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    #[cfg(target_arch = "wasm32")]
    let threads = 1usize;

    if threads > 1 && rows * cols >= 512 {
        let rows_per_chunk = (rows + threads - 1) / threads;
        let chunk_elems = rows_per_chunk * cols;
        std::thread::scope(|s| {
            for (chunk_idx, chunk_slice) in heights.chunks_mut(chunk_elems).enumerate() {
                let row_start = chunk_idx * rows_per_chunk;
                s.spawn(move || {
                    for (local_r, row_slice) in chunk_slice.chunks_mut(cols).enumerate() {
                        eval_row(row_start + local_r, row_slice);
                    }
                });
            }
        });
    } else {
        for (iz, row_slice) in heights.chunks_mut(cols).enumerate() {
            eval_row(iz, row_slice);
        }
    }

    heights
}

/// Evaluates the full 3D ground-terrain vertex grid: support-polygon clipping,
/// organic height noise, building box-SDF flattening, and track/road corridor carving.
#[allow(clippy::too_many_arguments)]
pub fn carve_ground_terrain_grid(
    segs_x: usize,
    segs_z: usize,
    width: f64,
    depth: f64,
    max_height: f64,
    base_layer_elev: f64,
    model_x: f64,
    model_z: f64,
    cos_m: f64,
    sin_m: f64,
    transition_dist: f64,
    is_water: bool,
    r_dist_grid: &[f32],
    support_poly_offsets: &[u32],
    support_poly_coords: &[f32],
    buildings: &[f32],
    corridor_meta: &[f32],
    corridor_pts: &[f32],
) -> Vec<f32> {
    let cols = segs_x + 1;
    let rows = segs_z + 1;
    let total = cols * rows;
    let mut out = vec![0.0_f32; total * 4];
    let clip_to_support = support_poly_offsets.len() >= 2;
    let half_w = width * 0.5;
    let half_d = depth * 0.5;
    let inv_sx = if segs_x > 0 { 1.0 / segs_x as f64 } else { 0.0 };
    let inv_sz = if segs_z > 0 { 1.0 / segs_z as f64 } else { 0.0 };

    let transition_sq = transition_dist * transition_dist;

    struct GroundBldg {
        wx: f64,
        wz: f64,
        cos: f64,
        sin: f64,
        hw: f64,
        hd: f64,
        max_r: f64,
    }
    let mut bldgs = Vec::with_capacity(buildings.len() / 6);
    let mut bi = 0;
    while bi + 5 < buildings.len() {
        let b_hw = buildings[bi + 4] as f64;
        let b_hd = buildings[bi + 5] as f64;
        bldgs.push(GroundBldg {
            wx: buildings[bi] as f64,
            wz: buildings[bi + 1] as f64,
            cos: buildings[bi + 2] as f64,
            sin: buildings[bi + 3] as f64,
            hw: b_hw,
            hd: b_hd,
            max_r: b_hw.max(b_hd) + transition_dist,
        });
        bi += 6;
    }

    struct GroundCorridor {
        min_x: f64,
        max_x: f64,
        min_z: f64,
        max_z: f64,
        c_hw: f64,
        c_hw_sq: f64,
        r_outer: f64,
        r_outer_sq: f64,
        is_tt: bool,
        pts: Vec<[f64; 2]>,
        g_min_x: f64,
        g_min_z: f64,
        inv_cell: f64,
        grid_w: usize,
        grid_h: usize,
        cells: Vec<Vec<usize>>,
    }
    let mut prepared_corridors = Vec::with_capacity(corridor_meta.len() / 8);
    let mut ci = 0;
    while ci + 7 < corridor_meta.len() {
        let c_hw = corridor_meta[ci + 4] as f64;
        let r_outer = c_hw + transition_dist;
        let is_tt = corridor_meta[ci + 5] > 0.5;
        let pt_start = corridor_meta[ci + 6] as usize;
        let pt_count = corridor_meta[ci + 7] as usize;
        let mut pts = Vec::with_capacity(pt_count);
        let (mut pmin_x, mut pmax_x, mut pmin_z, mut pmax_z) =
            (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for p in 0..pt_count {
            let pb = (pt_start + p) * 3;
            if pb + 2 >= corridor_pts.len() {
                break;
            }
            let pt_elev = corridor_pts[pb + 2] as f64;
            if (pt_elev - base_layer_elev).abs() > 45.0 {
                continue;
            }
            let px = corridor_pts[pb] as f64;
            let pz = corridor_pts[pb + 1] as f64;
            pmin_x = pmin_x.min(px);
            pmax_x = pmax_x.max(px);
            pmin_z = pmin_z.min(pz);
            pmax_z = pmax_z.max(pz);
            pts.push([px, pz]);
        }
        if !pts.is_empty() {
            let (inv_cell, grid_w, grid_h, cells) = if pts.len() > 12 {
                let min_cell = (r_outer * 1.05 + 1.0).max(24.0);
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
            prepared_corridors.push(GroundCorridor {
                min_x: corridor_meta[ci] as f64,
                max_x: corridor_meta[ci + 1] as f64,
                min_z: corridor_meta[ci + 2] as f64,
                max_z: corridor_meta[ci + 3] as f64,
                c_hw,
                c_hw_sq: c_hw * c_hw,
                r_outer,
                r_outer_sq: r_outer * r_outer,
                is_tt,
                pts,
                g_min_x: pmin_x,
                g_min_z: pmin_z,
                inv_cell,
                grid_w,
                grid_h,
                cells,
            });
        }
        ci += 8;
    }

    let eval_carve_row = |iz: usize, row_out: &mut [f32]| {
        let lz = (iz as f64 * inv_sz - 0.5) * depth;
        let nz = if half_d > 0.0 { lz / half_d } else { 0.0 };

        for ix in 0..cols {
            let v_idx = iz * cols + ix;
            let cell_base = ix * 4;
            let lx = (ix as f64 * inv_sx - 0.5) * width;
            let nx = if half_w > 0.0 { lx / half_w } else { 0.0 };

            let wx = model_x + (lx * cos_m - lz * sin_m);
            let wz = model_z + (lx * sin_m + lz * cos_m);

            let in_bounds = !clip_to_support
                || is_point_in_any_polygon(wx, wz, support_poly_offsets, support_poly_coords);

            if !in_bounds {
                row_out[cell_base] = 0.0;
                row_out[cell_base + 1] = 0.0;
                row_out[cell_base + 2] = 0.0;
                row_out[cell_base + 3] = 1.0;
                continue;
            }

            let r_dist = r_dist_grid.get(v_idx).copied().unwrap_or(nx.hypot(nz) as f32) as f64;
            let edge_fade = 1.0 - smoothstep(0.68, 1.0, r_dist);
            let n_val = mountain_noise(nx * 1.4, nz * 1.4);
            let wave = 0.55 + 0.45 * (nx * 2.4 + 0.7).sin() * (nz * 2.1 - 0.4).cos() + n_val * 0.25;
            let mut h = if is_water {
                edge_fade * 2.2
            } else {
                (edge_fade * max_height * wave.max(0.25)).max(0.0)
            };

            let mut flatten_weight = 0.0_f64;
            let mut inside_turntable = false;

            for b in &bldgs {
                let dx = wx - b.wx;
                if dx.abs() > b.max_r {
                    continue;
                }
                let dz = wz - b.wz;
                if dz.abs() > b.max_r {
                    continue;
                }
                let blx = dx * b.cos - dz * b.sin;
                let blz = dx * b.sin + dz * b.cos;
                let qx = (blx.abs() - b.hw).max(0.0);
                let qz = (blz.abs() - b.hd).max(0.0);
                let d_sq = qx * qx + qz * qz;
                if d_sq < transition_sq {
                    let w = 1.0 - smoothstep(0.0, transition_dist, d_sq.sqrt());
                    if w > flatten_weight {
                        flatten_weight = w;
                    }
                }
            }

            for col in &prepared_corridors {
                if wx < col.min_x || wx > col.max_x || wz < col.min_z || wz > col.max_z {
                    continue;
                }
                if col.cells.is_empty() {
                    for pt in &col.pts {
                        let dx = wx - pt[0];
                        let dz = wz - pt[1];
                        let d_sq = dx * dx + dz * dz;
                        if col.is_tt && d_sq < col.c_hw_sq {
                            inside_turntable = true;
                            flatten_weight = 1.0;
                            break;
                        }
                        if d_sq < col.r_outer_sq {
                            let w = 1.0 - smoothstep(col.c_hw, col.r_outer, d_sq.sqrt());
                            if w > flatten_weight {
                                flatten_weight = w;
                            }
                        }
                    }
                } else {
                    let qcx = ((wx - col.g_min_x) * col.inv_cell).floor() as isize;
                    let qcz = ((wz - col.g_min_z) * col.inv_cell).floor() as isize;
                    let x0 = (qcx - 1).max(0);
                    let x1 = (qcx + 1).min(col.grid_w as isize - 1);
                    let z0 = (qcz - 1).max(0);
                    let z1 = (qcz + 1).min(col.grid_h as isize - 1);
                    for gz in z0..=z1 {
                        for gx in x0..=x1 {
                            for &pi in &col.cells[gz as usize * col.grid_w + gx as usize] {
                                let pt = col.pts[pi];
                                let dx = wx - pt[0];
                                let dz = wz - pt[1];
                                let d_sq = dx * dx + dz * dz;
                                if col.is_tt && d_sq < col.c_hw_sq {
                                    inside_turntable = true;
                                    flatten_weight = 1.0;
                                    break;
                                }
                                if d_sq < col.r_outer_sq {
                                    let w = 1.0 - smoothstep(col.c_hw, col.r_outer, d_sq.sqrt());
                                    if w > flatten_weight {
                                        flatten_weight = w;
                                    }
                                }
                            }
                            if inside_turntable {
                                break;
                            }
                        }
                        if inside_turntable {
                            break;
                        }
                    }
                }
                if inside_turntable {
                    break;
                }
            }

            let is_water_vert = if inside_turntable {
                h = 0.0;
                false
            } else if is_water {
                let iw = edge_fade > 0.15 && flatten_weight < 0.45;
                h = if iw {
                    0.35
                } else {
                    edge_fade * (1.0 - flatten_weight * 0.85) * max_height.min(5.0)
                };
                iw
            } else {
                h *= 1.0 - flatten_weight;
                false
            };

            row_out[cell_base] = h as f32;
            row_out[cell_base + 1] = 1.0;
            row_out[cell_base + 2] = if is_water_vert { 1.0 } else { 0.0 };
            row_out[cell_base + 3] = flatten_weight as f32;
        }
    };

    let row_stride = cols * 4;
    #[cfg(not(target_arch = "wasm32"))]
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    #[cfg(target_arch = "wasm32")]
    let threads = 1usize;

    if threads > 1 && total >= 512 {
        let rows_per_chunk = (rows + threads - 1) / threads;
        let chunk_elems = rows_per_chunk * row_stride;
        std::thread::scope(|s| {
            for (chunk_idx, chunk_slice) in out.chunks_mut(chunk_elems).enumerate() {
                let row_start = chunk_idx * rows_per_chunk;
                s.spawn(move || {
                    for (local_r, row_slice) in chunk_slice.chunks_mut(row_stride).enumerate() {
                        eval_carve_row(row_start + local_r, row_slice);
                    }
                });
            }
        });
    } else {
        for (iz, row_slice) in out.chunks_mut(row_stride).enumerate() {
            eval_carve_row(iz, row_slice);
        }
    }

    out
}

/// Batch world-to-local transform + mountain footprint `norm_r` & surface height evaluation.
pub fn evaluate_mountain_track_samples(
    pts_xye: &[f32],
    m_x: f64,
    m_z: f64,
    cos_inv: f64,
    sin_inv: f64,
    half_w: f64,
    half_d: f64,
    height: f64,
) -> Vec<f32> {
    let n = pts_xye.len() / 3;
    let mut out = vec![0.0_f32; n * 2];
    if half_w <= 0.0 || half_d <= 0.0 {
        return out;
    }

    for i in 0..n {
        let dx = pts_xye[i * 3] as f64 - m_x;
        let dz = pts_xye[i * 3 + 1] as f64 - m_z;
        let lx = dx * cos_inv - dz * sin_inv;
        let lz = dx * sin_inv + dz * cos_inv;
        let nx = lx / half_w;
        let nz = lz / half_d;
        let norm_r = nx.hypot(nz);
        let surf_h = natural_mountain_height_at(nx, nz, height);
        out[i * 2] = norm_r as f32;
        out[i * 2 + 1] = surf_h as f32;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mountain_heightfield_center_peak() {
        let h = evaluate_mountain_heightfield(4, 4, 200.0, 200.0, 150.0, &[]);
        assert_eq!(h.len(), 25);
        assert!(h[2 * 5 + 2] > 50.0);
        assert_eq!(h[0], 0.0);
    }

    #[test]
    fn mountain_heightfield_ridge_capsule_bridges_between_nodes() {
        // Two nodes 300 mm apart with radius 100 mm: isolated circles would leave a 0-height gap at x=0,
        // whereas capsule interpolation produces a smooth connecting mountain ridge at the center.
        let strokes = [
            -150.0_f32, 0.0, 100.0, 200.0,
             150.0_f32, 0.0, 100.0, 200.0,
        ];
        let h = evaluate_mountain_heightfield(6, 4, 250.0, 150.0, 200.0, &strokes);
        // Center grid vertex is iz = 2, ix = 3 (lx = 0, lz = 0)
        let mid_h = h[2 * 7 + 3];
        assert!(mid_h > 120.0, "Expected ridge capsule height > 120 at midpoint, got {mid_h}");
    }

    #[test]
    fn carve_ground_terrain_grid_flattens_dense_corridors_and_turntable() {
        let mut pts = Vec::new();
        for i in 0..24 {
            let x = -200.0 + (i as f32) * (400.0 / 23.0);
            pts.push(x);
            pts.push(0.0);
            pts.push(0.0);
        }
        // Dense non-turntable corridor along z = 0 plus turntable hole at (120, 0)
        let meta = [
            -260.0_f32, 260.0, -80.0, 80.0, 20.0, 0.0, 0.0, 24.0,
            60.0, 180.0, -60.0, 60.0, 35.0, 1.0, 18.0, 1.0,
        ];
        let buildings = [0.0_f32, 100.0, 1.0, 0.0, 20.0, 20.0];
        let out = carve_ground_terrain_grid(
            8, 8, 500.0, 500.0, 30.0, 0.0, 0.0, 0.0, 1.0, 0.0, 40.0, false,
            &[], &[], &[], &buildings, &meta, &pts,
        );
        assert_eq!(out.len(), 9 * 9 * 4);
        // Center vertex (iz=4, ix=4 -> wx=0, wz=0) sits directly on corridor: height ~ 0, flatten_weight ~ 1
        let center_idx = (4 * 9 + 4) * 4;
        assert!(out[center_idx] < 1.0, "expected corridor center carved flat, got {}", out[center_idx]);
        assert!(out[center_idx + 3] > 0.99);
    }
}
