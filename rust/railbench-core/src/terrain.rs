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

    for iz in 0..rows {
        let vz = iz as f64 * inv_nz;
        let lz = (vz * 2.0 - 1.0) * half_d;
        let nz = lz / half_d;

        for ix in 0..cols {
            let vx = ix as f64 * inv_nx;
            let lx = (vx * 2.0 - 1.0) * half_w;
            let nx = lx / half_w;

            let h = if !has_strokes {
                natural_mountain_height_at(nx, nz, height)
            } else {
                let mut best = 0.0_f64;
                let mut s = 0;
                while s + 3 < strokes.len() {
                    let sx = strokes[s] as f64;
                    let sz = strokes[s + 1] as f64;
                    let sr = (strokes[s + 2] as f64).max(1.0);
                    let sh = strokes[s + 3] as f64;
                    let d = (lx - sx).hypot(lz - sz) / sr;
                    if d < 1.0 {
                        let prof = (d * std::f64::consts::FRAC_PI_2).cos().powf(1.35);
                        let n = mountain_noise(lx / sr, lz / sr);
                        let cand = (sh * prof * (0.88 + 0.16 * n)).max(0.0);
                        if cand > best {
                            best = cand;
                        }
                    }
                    s += 4;
                }
                best
            };

            heights[iz * cols + ix] = h as f32;
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

    for iz in 0..rows {
        let lz = (iz as f64 * inv_sz - 0.5) * depth;
        let nz = if half_d > 0.0 { lz / half_d } else { 0.0 };

        for ix in 0..cols {
            let v_idx = iz * cols + ix;
            let lx = (ix as f64 * inv_sx - 0.5) * width;
            let nx = if half_w > 0.0 { lx / half_w } else { 0.0 };

            let wx = model_x + (lx * cos_m - lz * sin_m);
            let wz = model_z + (lx * sin_m + lz * cos_m);

            let in_bounds = !clip_to_support
                || is_point_in_any_polygon(wx, wz, support_poly_offsets, support_poly_coords);

            if !in_bounds {
                out[v_idx * 4] = 0.0;
                out[v_idx * 4 + 1] = 0.0;
                out[v_idx * 4 + 2] = 0.0;
                out[v_idx * 4 + 3] = 1.0;
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

            let mut bi = 0;
            while bi + 5 < buildings.len() {
                let b_wx = buildings[bi] as f64;
                let b_wz = buildings[bi + 1] as f64;
                let b_cos = buildings[bi + 2] as f64;
                let b_sin = buildings[bi + 3] as f64;
                let b_hw = buildings[bi + 4] as f64;
                let b_hd = buildings[bi + 5] as f64;
                let dx = wx - b_wx;
                let dz = wz - b_wz;
                let max_r = b_hw.max(b_hd) + transition_dist;
                if dx.abs() <= max_r && dz.abs() <= max_r {
                    let blx = dx * b_cos - dz * b_sin;
                    let blz = dx * b_sin + dz * b_cos;
                    let qx = (blx.abs() - b_hw).max(0.0);
                    let qz = (blz.abs() - b_hd).max(0.0);
                    let d = qx.hypot(qz);
                    if d < transition_dist {
                        let w = 1.0 - smoothstep(0.0, transition_dist, d);
                        if w > flatten_weight {
                            flatten_weight = w;
                        }
                    }
                }
                bi += 6;
            }

            let mut ci = 0;
            while ci + 7 < corridor_meta.len() {
                let min_x = corridor_meta[ci] as f64;
                let max_x = corridor_meta[ci + 1] as f64;
                let min_z = corridor_meta[ci + 2] as f64;
                let max_z = corridor_meta[ci + 3] as f64;
                if wx >= min_x && wx <= max_x && wz >= min_z && wz <= max_z {
                    let c_hw = corridor_meta[ci + 4] as f64;
                    let is_tt = corridor_meta[ci + 5] > 0.5;
                    let pt_start = corridor_meta[ci + 6] as usize;
                    let pt_count = corridor_meta[ci + 7] as usize;
                    for p in 0..pt_count {
                        let pb = (pt_start + p) * 3;
                        if pb + 2 >= corridor_pts.len() {
                            break;
                        }
                        let pt_elev = corridor_pts[pb + 2] as f64;
                        if (pt_elev - base_layer_elev).abs() > 45.0 {
                            continue;
                        }
                        let d = (wx - corridor_pts[pb] as f64).hypot(wz - corridor_pts[pb + 1] as f64);
                        if is_tt && d < c_hw {
                            inside_turntable = true;
                            flatten_weight = 1.0;
                            break;
                        }
                        if d < c_hw + transition_dist {
                            let w = 1.0 - smoothstep(c_hw, c_hw + transition_dist, d);
                            if w > flatten_weight {
                                flatten_weight = w;
                            }
                        }
                    }
                    if inside_turntable {
                        break;
                    }
                }
                ci += 8;
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

            out[v_idx * 4] = h as f32;
            out[v_idx * 4 + 1] = 1.0;
            out[v_idx * 4 + 2] = if is_water_vert { 1.0 } else { 0.0 };
            out[v_idx * 4 + 3] = flatten_weight as f32;
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
}
