use crate::types::Vec2;

#[inline]
pub fn deg2rad(deg: f64) -> f64 {
    deg * (std::f64::consts::PI / 180.0)
}

#[inline]
pub fn rad2deg(rad: f64) -> f64 {
    rad * (180.0 / std::f64::consts::PI)
}

#[inline]
pub fn norm_angle_deg(a: f64) -> f64 {
    let r = a % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

#[inline]
pub fn dist(a: Vec2, b: Vec2) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

#[inline]
pub fn rotate(p: Vec2, angle_deg: f64) -> Vec2 {
    let rad = deg2rad(angle_deg);
    let c = rad.cos();
    let s = rad.sin();
    Vec2 {
        x: p.x * c - p.y * s,
        y: p.x * s + p.y * c,
    }
}

#[inline]
pub fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
pub fn is_point_in_any_polygon(
    x: f64,
    y: f64,
    poly_offsets: &[u32],
    poly_coords: &[f32],
) -> bool {
    if poly_offsets.len() < 2 {
        return true;
    }
    let poly_count = poly_offsets.len() - 1;
    for p in 0..poly_count {
        let start = poly_offsets[p] as usize;
        let end = poly_offsets[p + 1] as usize;
        let n = end.saturating_sub(start);
        if n < 3 {
            continue;
        }
        let mut inside = false;
        let mut j = n - 1;
        for i in 0..n {
            let xi = poly_coords[(start + i) * 2] as f64;
            let yi = poly_coords[(start + i) * 2 + 1] as f64;
            let xj = poly_coords[(start + j) * 2] as f64;
            let yj = poly_coords[(start + j) * 2 + 1] as f64;
            let intersect = ((yi > y) != (yj > y)) && (x < ((xj - xi) * (y - yi)) / (yj - yi) + xi);
            if intersect {
                inside = !inside;
            }
            j = i;
        }
        if inside {
            return true;
        }
    }
    false
}

/// Evaluates the cubic Hermite flextrack spline and returns flat `Vec<f64>` of stride 5 per sample:
/// `[x, y, nx, ny, tangent_angle_rad]`.
pub fn sample_flex_spline_points(
    chord_dist: f64,
    h1: f64,
    h2: f64,
    h3: f64,
    num_samples: usize,
) -> Vec<f64> {
    let count = num_samples + 1;
    let mut out = Vec::with_capacity(count * 5);
    let w = 0.25_f64;
    let m1 = 2.0 * h2;
    let m2 = 2.0 * (h3 - h1);
    let m3 = -2.0 * h2;
    let m0 = 2.0 * (4.0 * h1 - h2);
    let m4 = -2.0 * (4.0 * h3 - h2);
    let inv_n = if num_samples > 0 {
        1.0 / num_samples as f64
    } else {
        0.0
    };

    for step in 0..count {
        let t = (step as f64 * inv_n).clamp(0.0, 1.0);
        let seg = if t <= 0.25 {
            0
        } else if t <= 0.5 {
            1
        } else if t <= 0.75 {
            2
        } else {
            3
        };
        let u = ((t - seg as f64 * w) / w).clamp(0.0, 1.0);
        let (y_a, y_b, m_a, m_b) = match seg {
            0 => (0.0, h1, m0, m1),
            1 => (h1, h2, m1, m2),
            2 => (h2, h3, m2, m3),
            _ => (h3, 0.0, m3, m4),
        };
        let s_a = m_a * w;
        let s_b = m_b * w;
        let u2 = u * u;
        let u3 = u2 * u;

        let h0_b = 2.0 * u3 - 3.0 * u2 + 1.0;
        let h1_b = u3 - 2.0 * u2 + u;
        let h2_b = -2.0 * u3 + 3.0 * u2;
        let h3_b = u3 - u2;
        let h = h0_b * y_a + h1_b * s_a + h2_b * y_b + h3_b * s_b;

        let dh0 = 6.0 * u2 - 6.0 * u;
        let dh1 = 3.0 * u2 - 4.0 * u + 1.0;
        let dh2 = -6.0 * u2 + 6.0 * u;
        let dh3 = 3.0 * u2 - 2.0 * u;
        let dh = (dh0 * y_a + dh1 * s_a + dh2 * y_b + dh3 * s_b) / w;

        let x = t * chord_dist;
        let tangent = dh.atan2(chord_dist);
        let nx = -tangent.sin();
        let ny = tangent.cos();

        out.push(x);
        out.push(h);
        out.push(nx);
        out.push(ny);
        out.push(tangent);
    }

    out
}

/// Sweeps a 2D tunnel cross-section along a 3D centerline path into flat `Vec<f32>` vertex positions.
pub fn build_tunnel_sweep_positions(path_xyz: &[f32], section_xy: &[f32]) -> Vec<f32> {
    let n_path = path_xyz.len() / 3;
    let n_sec = section_xy.len() / 2;
    if n_path < 2 || n_sec == 0 {
        return Vec::new();
    }

    let mut positions = Vec::with_capacity(n_path * n_sec * 3);
    let mut prev_sx = 1.0_f64;
    let mut prev_sz = 0.0_f64;

    for i in 0..n_path {
        let ia = i.saturating_sub(1);
        let ib = (i + 1).min(n_path - 1);
        let tx = (path_xyz[ib * 3] - path_xyz[ia * 3]) as f64;
        let tz = (path_xyz[ib * 3 + 2] - path_xyz[ia * 3 + 2]) as f64;
        let len = tx.hypot(tz);
        let (sx, sz) = if len > 1e-6 {
            (tz / len, -tx / len)
        } else {
            (prev_sx, prev_sz)
        };
        prev_sx = sx;
        prev_sz = sz;

        let cx = path_xyz[i * 3] as f64;
        let cy = path_xyz[i * 3 + 1] as f64;
        let cz = path_xyz[i * 3 + 2] as f64;

        for j in 0..n_sec {
            let x = section_xy[j * 2] as f64;
            let y = section_xy[j * 2 + 1] as f64;
            positions.push((cx + sx * x) as f32);
            positions.push((cy + y) as f32);
            positions.push((cz + sz * x) as f32);
        }
    }

    positions
}

/// Clamps, rounds (0.1 mm), deduplicates micro-segments, and removes collinear points
/// from a 2D table-cutout polygon (`raw_xy` stride 2).
pub fn simplify_cutout_polygon(
    raw_xy: &[f64],
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    dedup_dist: f64,
    collinear_tol: f64,
) -> Vec<f64> {
    let n_raw = raw_xy.len() / 2;
    if n_raw < 3 {
        return Vec::new();
    }

    let mut dedup: Vec<(f64, f64)> = Vec::with_capacity(n_raw);
    for i in 0..n_raw {
        let rx = (raw_xy[i * 2] * 10.0).round() * 0.1;
        let ry = (raw_xy[i * 2 + 1] * 10.0).round() * 0.1;
        let cx = rx.clamp(min_x, max_x);
        let cy = ry.clamp(min_y, max_y);
        if let Some(&(lx, ly)) = dedup.last() {
            if (cx - lx).hypot(cy - ly) < dedup_dist {
                continue;
            }
        }
        dedup.push((cx, cy));
    }

    let n = dedup.len();
    let mut simplified: Vec<(f64, f64)> = if n <= 3 {
        dedup
    } else {
        let mut res = Vec::with_capacity(n);
        for i in 0..n {
            let prev = res.last().copied().unwrap_or(dedup[(i + n - 1) % n]);
            let cur = dedup[i];
            let next = dedup[(i + 1) % n];
            let dx1 = cur.0 - prev.0;
            let dy1 = cur.1 - prev.1;
            let dx2 = next.0 - cur.0;
            let dy2 = next.1 - cur.1;
            let cross = (dx1 * dy2 - dy1 * dx2).abs();
            let len1 = dx1.hypot(dy1);
            let len2 = dx2.hypot(dy2);
            let denom =
                (if len1 == 0.0 { 1.0 } else { len1 }) * (if len2 == 0.0 { 1.0 } else { len2 });
            if cross / denom > collinear_tol || res.len() < 2 {
                res.push(cur);
            }
        }
        res
    };

    if simplified.len() > 2 {
        let first = simplified[0];
        let last = *simplified.last().unwrap();
        if (first.0 - last.0).hypot(first.1 - last.1) <= 2.0 {
            simplified.pop();
        }
    }

    let mut out = Vec::with_capacity(simplified.len() * 2);
    for (x, y) in simplified {
        out.push(x);
        out.push(y);
    }
    out
}
