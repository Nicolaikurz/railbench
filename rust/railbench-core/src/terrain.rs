use std::sync::Arc;

use crate::geometry::{is_point_in_any_polygon, smoothstep};
use crate::types::TerrainHeightfield;

pub const BIOME_AUTO: u8 = 0;
pub const BIOME_GRASS: u8 = 1;
pub const BIOME_FOREST: u8 = 2;
pub const BIOME_ROCK: u8 = 3;
pub const BIOME_SNOW: u8 = 4;
pub const BIOME_SAND: u8 = 5;
pub const BIOME_DIRT: u8 = 6;
pub const BIOME_WATER: u8 = 7;

#[inline]
pub fn mountain_noise(nx: f64, nz: f64) -> f64 {
    let s1 = (nx * std::f64::consts::PI + 0.5).sin() * (nz * std::f64::consts::PI + 0.3).cos();
    let s2 = (nx * std::f64::consts::TAU + nz * 4.71).sin() * 0.35;
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
        let rows_per_chunk = rows.div_ceil(threads);
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
        let rows_per_chunk = rows.div_ceil(threads);
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
#[allow(clippy::too_many_arguments)]
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

/// Multi-octave ridged + fractional Brownian noise in `[0.0, 1.0]` for organic terrain sculpting.
#[inline]
pub fn ridged_fbm_noise(x: f64, z: f64) -> f64 {
    let f1 = (x * std::f64::consts::PI + 0.5).sin() * (z * std::f64::consts::PI + 0.3).cos();
    let f2 = (x * 2.13 * std::f64::consts::PI + z * 1.47).sin() * 0.5;
    let f3 = (x * 4.31 - z * 3.79).cos() * 0.25;
    let raw = f1 + f2 + f3; // approx [-1.75, 1.75]
    let ridged = 1.0 - (f1.abs() * 0.65 + (f2 * 0.7).abs() * 0.35).clamp(0.0, 1.0);
    let smooth = (raw * 0.28 + 0.5).clamp(0.0, 1.0);
    (0.55 * smooth + 0.45 * ridged).clamp(0.0, 1.0)
}

pub fn create_heightfield(
    min_x: f64,
    min_z: f64,
    max_x: f64,
    max_z: f64,
    target_cell_mm: f64,
) -> TerrainHeightfield {
    let w = (max_x - min_x).max(100.0);
    let d = (max_z - min_z).max(100.0);
    let cell = target_cell_mm.max(8.0);
    let cols = (w / cell).round().clamp(32.0, 128.0) as usize + 1;
    let rows = (d / cell).round().clamp(32.0, 128.0) as usize + 1;
    let n = cols * rows;
    TerrainHeightfield {
        cols,
        rows,
        min_x,
        min_z,
        max_x: min_x + w,
        max_z: min_z + d,
        heights: Arc::new(vec![0.0_f32; n]),
        materials: Arc::new(vec![BIOME_AUTO; n]),
        revision: 1,
    }
}

pub fn sample_heightfield_bilinear(hf: &TerrainHeightfield, wx: f64, wz: f64) -> f32 {
    if hf.cols < 2 || hf.rows < 2 || hf.heights.len() < hf.cols * hf.rows {
        return 0.0;
    }
    let span_x = hf.max_x - hf.min_x;
    let span_z = hf.max_z - hf.min_z;
    if span_x <= 1e-6 || span_z <= 1e-6 {
        return 0.0;
    }
    if wx < hf.min_x || wx > hf.max_x || wz < hf.min_z || wz > hf.max_z {
        return 0.0;
    }
    let fx = ((wx - hf.min_x) / span_x).clamp(0.0, 1.0) * (hf.cols - 1) as f64;
    let fz = ((wz - hf.min_z) / span_z).clamp(0.0, 1.0) * (hf.rows - 1) as f64;
    let x0 = (fx.floor() as usize).min(hf.cols - 2);
    let z0 = (fz.floor() as usize).min(hf.rows - 2);
    let tx = (fx - x0 as f64) as f32;
    let tz = (fz - z0 as f64) as f32;
    let h00 = hf.heights[z0 * hf.cols + x0];
    let h10 = hf.heights[z0 * hf.cols + x0 + 1];
    let h01 = hf.heights[(z0 + 1) * hf.cols + x0];
    let h11 = hf.heights[(z0 + 1) * hf.cols + x0 + 1];
    let a = h00 + (h10 - h00) * tx;
    let b = h01 + (h11 - h01) * tx;
    a + (b - a) * tz
}

pub fn sample_heightfield_material(hf: &TerrainHeightfield, wx: f64, wz: f64) -> u8 {
    if hf.cols < 2 || hf.rows < 2 || hf.materials.len() < hf.cols * hf.rows {
        return BIOME_AUTO;
    }
    let span_x = hf.max_x - hf.min_x;
    let span_z = hf.max_z - hf.min_z;
    if span_x <= 1e-6 || span_z <= 1e-6 {
        return BIOME_AUTO;
    }
    if wx < hf.min_x || wx > hf.max_x || wz < hf.min_z || wz > hf.max_z {
        return BIOME_AUTO;
    }
    let ix = (((wx - hf.min_x) / span_x).clamp(0.0, 1.0) * (hf.cols - 1) as f64)
        .round() as usize;
    let iz = (((wz - hf.min_z) / span_z).clamp(0.0, 1.0) * (hf.rows - 1) as f64)
        .round() as usize;
    hf.materials[iz.min(hf.rows - 1) * hf.cols + ix.min(hf.cols - 1)]
}

pub fn resample_heightfield(
    old: &TerrainHeightfield,
    min_x: f64,
    min_z: f64,
    max_x: f64,
    max_z: f64,
    target_cell_mm: f64,
) -> TerrainHeightfield {
    let mut fresh = create_heightfield(min_x, min_z, max_x, max_z, target_cell_mm);
    fresh.revision = old.revision.saturating_add(1);
    let span_x = fresh.max_x - fresh.min_x;
    let span_z = fresh.max_z - fresh.min_z;
    let h_mut = Arc::make_mut(&mut fresh.heights);
    let m_mut = Arc::make_mut(&mut fresh.materials);
    for iz in 0..fresh.rows {
        let wz = fresh.min_z + (iz as f64 / (fresh.rows - 1).max(1) as f64) * span_z;
        for ix in 0..fresh.cols {
            let wx = fresh.min_x + (ix as f64 / (fresh.cols - 1).max(1) as f64) * span_x;
            let idx = iz * fresh.cols + ix;
            h_mut[idx] = sample_heightfield_bilinear(old, wx, wz);
            m_mut[idx] = sample_heightfield_material(old, wx, wz);
        }
    }
    fresh
}

/// Raycasts a 3D viewport cursor ray `(origin, dir)` against the sculpted `TerrainHeightfield`
/// inside `(min_x, min_z, max_x, max_z)`, returning the exact 3D surface hit `[x, y, z]`.
pub fn raycast_heightfield_3d(
    origin: [f32; 3],
    dir: [f32; 3],
    hf: Option<&TerrainHeightfield>,
    bounds: (f64, f64, f64, f64),
) -> Option<[f32; 3]> {
    let (bmin_x, bmin_z, bmax_x, bmax_z) = bounds;
    let hit_plane_fallback = || -> Option<[f32; 3]> {
        if dir[1].abs() < 1e-4 {
            return None;
        }
        let t = -origin[1] / dir[1];
        if t <= 0.5 || t > 100_000.0 {
            return None;
        }
        let hx = origin[0] + dir[0] * t;
        let hz = origin[2] + dir[2] * t;
        if (hx as f64) < bmin_x
            || (hx as f64) > bmax_x
            || (hz as f64) < bmin_z
            || (hz as f64) > bmax_z
        {
            return None;
        }
        Some([hx, 0.0, hz])
    };

    let Some(hf) = hf else {
        return hit_plane_fallback();
    };

    // Slab intersection with [min_x..max_x, -40..600, min_z..max_z]
    let min_b = [bmin_x as f32, -40.0_f32, bmin_z as f32];
    let max_b = [bmax_x as f32, 600.0_f32, bmax_z as f32];
    let mut tmin = 0.5_f32;
    let mut tmax = 100_000.0_f32;
    for axis in 0..3 {
        if dir[axis].abs() < 1e-6 {
            if origin[axis] < min_b[axis] || origin[axis] > max_b[axis] {
                return hit_plane_fallback();
            }
        } else {
            let inv = 1.0 / dir[axis];
            let mut t1 = (min_b[axis] - origin[axis]) * inv;
            let mut t2 = (max_b[axis] - origin[axis]) * inv;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return hit_plane_fallback();
            }
        }
    }

    let steps = 320usize;
    let dt = ((tmax - tmin) / steps as f32).clamp(6.0, 28.0);
    let mut prev_t = tmin;
    let mut prev_above = true;
    let mut t = tmin;
    while t <= tmax + dt * 0.5 {
        let tc = t.min(tmax);
        let rx = (origin[0] + dir[0] * tc) as f64;
        let ry = origin[1] + dir[1] * tc;
        let rz = (origin[2] + dir[2] * tc) as f64;
        let surf_y = sample_heightfield_bilinear(hf, rx, rz);
        if ry <= surf_y {
            if !prev_above && (tc - tmin) < dt * 0.5 {
                return Some([rx as f32, surf_y, rz as f32]);
            }
            let mut lo = prev_t;
            let mut hi = tc;
            for _ in 0..7 {
                let mid = (lo + hi) * 0.5;
                let mx = (origin[0] + dir[0] * mid) as f64;
                let my = origin[1] + dir[1] * mid;
                let mz = (origin[2] + dir[2] * mid) as f64;
                if my <= sample_heightfield_bilinear(hf, mx, mz) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            let hx = (origin[0] + dir[0] * hi) as f64;
            let hz = (origin[2] + dir[2] * hi) as f64;
            let hy = sample_heightfield_bilinear(hf, hx, hz);
            return Some([hx as f32, hy, hz as f32]);
        }
        prev_t = tc;
        prev_above = ry > surf_y;
        t += dt;
    }

    hit_plane_fallback()
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

    #[test]
    fn heightfield_bilinear_sampling_and_raycast_hit_surface() {
        let mut hf = create_heightfield(-400.0, -300.0, 400.0, 300.0, 25.0);
        let cx = (hf.cols - 1) / 2;
        let cz = (hf.rows - 1) / 2;
        let hm = Arc::make_mut(&mut hf.heights);
        for iz in cz.saturating_sub(3)..=(cz + 3).min(hf.rows - 1) {
            for ix in cx.saturating_sub(3)..=(cx + 3).min(hf.cols - 1) {
                let d = (ix as f32 - cx as f32).hypot(iz as f32 - cz as f32) / 3.5;
                if d < 1.0 {
                    hm[iz * hf.cols + ix] = (1.0 - d) * 140.0;
                }
            }
        }
        let peak_h = sample_heightfield_bilinear(&hf, 0.0, 0.0);
        assert!(peak_h > 100.0, "expected sampled center peak > 100mm, got {peak_h}");

        let origin = [0.0_f32, 350.0, 220.0];
        let dir = [0.0_f32, -0.78, -0.62];
        let hit = raycast_heightfield_3d(origin, dir, Some(&hf), (-400.0, -300.0, 400.0, 300.0))
            .expect("ray hits raised terrain heightfield");
        assert!(hit[1] > 35.0, "raycast should stop on raised surface, got y={}", hit[1]);
    }

    #[test]
    fn heightfield_serialization_roundtrip_preserves_grid() {
        let mut hf = create_heightfield(-500.0, -400.0, 500.0, 400.0, 50.0);
        Arc::make_mut(&mut hf.heights)[5] = 88.5;
        Arc::make_mut(&mut hf.materials)[5] = BIOME_ROCK;
        let layout = crate::types::Layout {
            name: "HFSerde".into(),
            scale: "H0".into(),
            pieces: Vec::new(),
            models: None,
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
            terrain_field: Some(hf),
        };
        let json = serde_json::to_string(&layout).unwrap();
        let parsed: crate::types::Layout = serde_json::from_str(&json).unwrap();
        let p_hf = parsed.terrain_field.expect("terrain_field roundtrips");
        assert!((p_hf.heights[5] - 88.5).abs() < 1e-4);
        assert_eq!(p_hf.materials[5], BIOME_ROCK);

        let legacy_json = json.replace("\"terrainField\":", "\"unusedLegacyIgnored\":");
        let parsed_legacy: crate::types::Layout = serde_json::from_str(&legacy_json).unwrap();
        assert!(parsed_legacy.terrain_field.is_none());
    }
}
