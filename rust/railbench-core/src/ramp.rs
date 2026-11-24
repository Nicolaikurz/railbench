use std::collections::HashMap;

/// Computes per-piece start & end elevations along a ramp/helix chain.
pub fn compute_ramp_chain_elevations(
    piece_lengths: &[f64],
    is_curve_left: &[u8],
    h_start: f64,
    total_rise: f64,
    target_layer_base_h: f64,
) -> Vec<f64> {
    let n = piece_lengths.len();
    let mut out = vec![0.0_f64; n * 2];
    let total_len: f64 = piece_lengths.iter().copied().sum();
    let mut accum = 0.0_f64;

    for i in 0..n {
        let len = piece_lengths[i];
        let s_dist = accum;
        let e_dist = accum + len;
        accum = e_dist;

        let u0 = if total_len > 0.0 { s_dist / total_len } else { 0.0 };
        let u1 = if total_len > 0.0 { e_dist / total_len } else { 1.0 };

        let h_entry = h_start + total_rise * u0;
        let h_exit = h_start + total_rise * u1;

        let left = is_curve_left.get(i).copied().unwrap_or(0) != 0;
        let (h_c0, h_c1) = if left {
            (h_exit, h_entry)
        } else {
            (h_entry, h_exit)
        };

        out[i * 2] = ((h_c0 - target_layer_base_h) * 100.0).round() / 100.0;
        out[i * 2 + 1] = ((h_c1 - target_layer_base_h) * 100.0).round() / 100.0;
    }

    out
}

/// Batch spatial clearance check for 3D catenary masts against track centerline samples.
pub fn check_catenary_mast_clearances(
    track_samples: &[f32],
    candidates: &[f32],
    vertical_tol: f32,
) -> Vec<u8> {
    let n_tracks = track_samples.len() / 4;
    let n_cands = candidates.len() / 3;
    let mut blocked = vec![0_u8; n_cands];
    if n_tracks == 0 || n_cands == 0 {
        return blocked;
    }

    const CELL: f32 = 60.0;
    let inv_cell = 1.0 / CELL;
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::with_capacity(n_tracks);

    for i in 0..n_tracks {
        let b = i * 4;
        let cx = (track_samples[b] * inv_cell).floor() as i32;
        let cz = (track_samples[b + 2] * inv_cell).floor() as i32;
        grid.entry((cx, cz)).or_default().push(i);
    }

    for (i, slot) in blocked.iter_mut().enumerate().take(n_cands) {
        let cb = i * 3;
        let mx = candidates[cb];
        let my = candidates[cb + 1];
        let mz = candidates[cb + 2];
        let cx = (mx * inv_cell).floor() as i32;
        let cz = (mz * inv_cell).floor() as i32;

        'outer: for dx in -1..=1 {
            for dz in -1..=1 {
                if let Some(bucket) = grid.get(&(cx + dx, cz + dz)) {
                    for &ti in bucket {
                        let tb = ti * 4;
                        if (track_samples[tb + 1] - my).abs() > vertical_tol {
                            continue;
                        }
                        let ddx = mx - track_samples[tb];
                        let ddz = mz - track_samples[tb + 2];
                        let r = track_samples[tb + 3];
                        if ddx * ddx + ddz * ddz < r * r {
                            *slot = 1;
                            break 'outer;
                        }
                    }
                }
            }
        }
    }

    blocked
}
