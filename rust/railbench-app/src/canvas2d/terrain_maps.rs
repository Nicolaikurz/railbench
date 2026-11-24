use railbench_core::geometry::{is_point_in_any_polygon, rotate};
use railbench_core::types::{Model3D, Vec2};

use crate::terrain_tools::{
    clamp_point_to_support_polygons, clip_segment_to_support, evaluate_mountain_local,
    pack_support_polygons,
};

pub(super) struct CachedMountainMap2D {
    /// Translucent hypsometric elevation tint cells in world space: (4 corners, band 0..=3)
    pub(super) tint_quads: Vec<([Vec2; 4], u8)>,
    /// Marching-squares topographic contour line segments in world space: (p0, p1, is_index_contour)
    pub(super) contour_segs: Vec<(Vec2, Vec2, bool)>,
    /// Cartographic downhill slope hachures (Schraffen): (p_contour, p_downhill)
    pub(super) hachures: Vec<(Vec2, Vec2)>,
    /// Summit peak markers in world space: (pos, height_mm)
    pub(super) peaks: Vec<(Vec2, f64)>,
}

pub(super) struct CachedGroundMap2D {
    /// Clipped concentric terrain quads in world space: (4 corners, band 0=outer..=2=inner)
    pub(super) quads: Vec<([Vec2; 4], u8)>,
    /// Clipped shoreline / perimeter contour segments
    pub(super) contour_segs: Vec<(Vec2, Vec2)>,
    /// Clipped water surface ripple segments (only populated for `water`)
    pub(super) ripple_segs: Vec<(Vec2, Vec2)>,
}

fn clip_quad_to_support(
    corners: [Vec2; 4],
    polys: &[Vec<Vec2>],
    offsets: &[u32],
    coords: &[f32],
) -> Option<[Vec2; 4]> {
    if offsets.len() < 2 {
        return Some(corners);
    }
    let any_inside = corners
        .iter()
        .any(|p| is_point_in_any_polygon(p.x, p.y, offsets, coords));
    if !any_inside {
        return None;
    }
    let out = corners.map(|p| clamp_point_to_support_polygons(p, polys, offsets, coords));
    let cx = (out[0].x + out[1].x + out[2].x + out[3].x) * 0.25;
    let cy = (out[0].y + out[1].y + out[2].y + out[3].y) * 0.25;
    is_point_in_any_polygon(cx, cy, offsets, coords).then_some(out)
}

fn terrain_local_bounds(m: &Model3D) -> (f64, f64, f64, f64) {
    let sc = m.scale.unwrap_or(1.0).max(0.05);
    let hw = (m.width * sc * 0.5).max(30.0);
    let hd = (m.depth * sc * 0.5).max(30.0);
    let (mut min_lx, mut max_lx, mut min_ly, mut max_ly) =
        if let Some(nodes) = m.mountain_path.as_ref().filter(|p| !p.is_empty()) {
            let mut x0 = f64::INFINITY;
            let mut x1 = f64::NEG_INFINITY;
            let mut y0 = f64::INFINITY;
            let mut y1 = f64::NEG_INFINITY;
            for n in nodes {
                let r = n.radius.max(25.0) * sc;
                let nx = n.x * sc;
                let ny = n.y * sc;
                x0 = x0.min(nx - r);
                x1 = x1.max(nx + r);
                y0 = y0.min(ny - r);
                y1 = y1.max(ny + r);
            }
            (x0, x1, y0, y1)
        } else {
            (-hw, hw, -hd, hd)
        };
    let pad_x = ((max_lx - min_lx) * 0.08).max(8.0);
    let pad_y = ((max_ly - min_ly) * 0.08).max(8.0);
    min_lx -= pad_x;
    max_lx += pad_x;
    min_ly -= pad_y;
    max_ly += pad_y;
    (min_lx, max_lx, min_ly, max_ly)
}

pub(super) fn build_mountain_map_2d(
    m: &Model3D,
    support_polys: &[Vec<Vec2>],
) -> CachedMountainMap2D {
    let sc = m.scale.unwrap_or(1.0).max(0.05);
    let rot = m.transform.rotation;
    let (min_lx, max_lx, min_ly, max_ly) = terrain_local_bounds(m);
    let (offsets, coords) = pack_support_polygons(support_polys);
    let clip = offsets.len() >= 2;

    const N: usize = 34;
    let dx = (max_lx - min_lx) / (N - 1) as f64;
    let dy = (max_ly - min_ly) / (N - 1) as f64;
    let mut grid = vec![0.0_f64; N * N];
    let mut max_h = 0.0_f64;
    let mut best_peak = (0.0_f64, 0.0_f64);

    let to_world = |lx: f64, ly: f64| -> Vec2 {
        let rp = rotate(Vec2 { x: lx, y: ly }, rot);
        Vec2 {
            x: m.transform.x + rp.x,
            y: m.transform.y + rp.y,
        }
    };

    for iy in 0..N {
        let ly = min_ly + iy as f64 * dy;
        for ix in 0..N {
            let lx = min_lx + ix as f64 * dx;
            let wp = to_world(lx, ly);
            if clip && !is_point_in_any_polygon(wp.x, wp.y, &offsets, &coords) {
                continue;
            }
            let (_, h) = evaluate_mountain_local(lx, ly, m);
            grid[iy * N + ix] = h;
            if h > max_h {
                max_h = h;
                best_peak = (lx, ly);
            }
        }
    }

    if max_h < 1.0 {
        return CachedMountainMap2D {
            tint_quads: Vec::new(),
            contour_segs: Vec::new(),
            hachures: Vec::new(),
            peaks: Vec::new(),
        };
    }

    // 1. Clipped hypsometric elevation-band quads
    let mut tint_quads = Vec::with_capacity(512);
    for iy in 0..(N - 1) {
        let ly0 = min_ly + iy as f64 * dy;
        let ly1 = ly0 + dy;
        for ix in 0..(N - 1) {
            let h00 = grid[iy * N + ix];
            let h10 = grid[iy * N + ix + 1];
            let h11 = grid[(iy + 1) * N + ix + 1];
            let h01 = grid[(iy + 1) * N + ix];
            let frac = (h00 + h10 + h11 + h01) * 0.25 / max_h;
            if frac < 0.075 {
                continue;
            }
            let band = if frac >= 0.78 {
                3
            } else if frac >= 0.54 {
                2
            } else if frac >= 0.28 {
                1
            } else {
                0
            };
            let lx0 = min_lx + ix as f64 * dx;
            let lx1 = lx0 + dx;
            let raw = [
                to_world(lx0, ly0),
                to_world(lx1, ly0),
                to_world(lx1, ly1),
                to_world(lx0, ly1),
            ];
            if let Some(clipped) = clip_quad_to_support(raw, support_polys, &offsets, &coords) {
                tint_quads.push((clipped, band));
            }
        }
    }

    // 2. Clipped Marching-Squares Iso-Contours & downhill slope hachures (`Schraffen`)
    let levels: &[(f64, bool, bool)] = &[
        (0.08, true, false),
        (0.21, false, false),
        (0.35, true, true),
        (0.50, false, false),
        (0.65, true, true),
        (0.79, false, false),
        (0.91, true, false),
    ];
    let mut contour_segs = Vec::with_capacity(1024);
    let mut hachures = Vec::with_capacity(192);
    let tick_len = (dx.hypot(dy) * 0.62).clamp(4.5, 16.0);
    let interp = |a: f64, b: f64, iso: f64| -> f64 {
        let denom = b - a;
        if denom.abs() < 1e-9 {
            0.5
        } else {
            ((iso - a) / denom).clamp(0.0, 1.0)
        }
    };

    for &(frac, is_index, emit_hachures) in levels {
        let iso = max_h * frac;
        let mut seg_counter = 0usize;
        for iy in 0..(N - 1) {
            let ly0 = min_ly + iy as f64 * dy;
            let ly1 = ly0 + dy;
            for ix in 0..(N - 1) {
                let h00 = grid[iy * N + ix];
                let h10 = grid[iy * N + ix + 1];
                let h11 = grid[(iy + 1) * N + ix + 1];
                let h01 = grid[(iy + 1) * N + ix];
                let mask = ((h00 >= iso) as u8)
                    | (((h10 >= iso) as u8) << 1)
                    | (((h11 >= iso) as u8) << 2)
                    | (((h01 >= iso) as u8) << 3);
                if mask == 0 || mask == 15 {
                    continue;
                }
                let lx0 = min_lx + ix as f64 * dx;
                let lx1 = lx0 + dx;
                let e0 = (lx0 + interp(h00, h10, iso) * dx, ly0);
                let e1 = (lx1, ly0 + interp(h10, h11, iso) * dy);
                let e2 = (lx0 + interp(h01, h11, iso) * dx, ly1);
                let e3 = (lx0, ly0 + interp(h00, h01, iso) * dy);
                let pairs: &[((f64, f64), (f64, f64))] = match mask {
                    1 | 14 => &[(e3, e0)],
                    2 | 13 => &[(e0, e1)],
                    3 | 12 => &[(e3, e1)],
                    4 | 11 => &[(e1, e2)],
                    5 => &[(e3, e2), (e0, e1)],
                    6 | 9 => &[(e0, e2)],
                    7 | 8 => &[(e3, e2)],
                    10 => &[(e3, e0), (e1, e2)],
                    _ => &[],
                };
                for &(a, b) in pairs {
                    let wa = to_world(a.0, a.1);
                    let wb = to_world(b.0, b.1);
                    if let Some((ca, cb)) = clip_segment_to_support(wa, wb, &offsets, &coords) {
                        contour_segs.push((ca, cb, is_index));
                        if emit_hachures {
                            seg_counter += 1;
                            if seg_counter % 3 == 0 {
                                let gx = -((h10 - h00) + (h11 - h01)) * 0.5 / dx.max(1e-3);
                                let gy = -((h01 - h00) + (h11 - h10)) * 0.5 / dy.max(1e-3);
                                let glen = gx.hypot(gy);
                                if glen > 1e-4 {
                                    let mx = (a.0 + b.0) * 0.5;
                                    let my = (a.1 + b.1) * 0.5;
                                    let w_mid = to_world(mx, my);
                                    let w_tip = to_world(
                                        mx + (gx / glen) * tick_len,
                                        my + (gy / glen) * tick_len,
                                    );
                                    if let Some((ha, hb)) =
                                        clip_segment_to_support(w_mid, w_tip, &offsets, &coords)
                                    {
                                        hachures.push((ha, hb));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Summit peak markers strictly inside the support polygon
    let mut peaks = Vec::new();
    if let Some(nodes) = m.mountain_path.as_ref().filter(|p| p.len() >= 2) {
        let mut picked: Vec<(f64, f64, f64)> = Vec::new();
        for n in nodes {
            let nx = n.x * sc;
            let ny = n.y * sc;
            let wp = to_world(nx, ny);
            if clip && !is_point_in_any_polygon(wp.x, wp.y, &offsets, &coords) {
                continue;
            }
            let nh = evaluate_mountain_local(nx, ny, m).1;
            if nh >= max_h * 0.65
                && picked
                    .iter()
                    .all(|&(px, py, _)| (px - nx).hypot(py - ny) > 120.0)
            {
                picked.push((nx, ny, nh));
            }
        }
        for (px, py, ph) in picked {
            peaks.push((to_world(px, py), ph));
        }
    }
    if peaks.is_empty() {
        let bp = to_world(best_peak.0, best_peak.1);
        if !clip || is_point_in_any_polygon(bp.x, bp.y, &offsets, &coords) {
            peaks.push((bp, max_h));
        }
    }

    CachedMountainMap2D {
        tint_quads,
        contour_segs,
        hachures,
        peaks,
    }
}

pub(super) fn build_ground_map_2d(
    m: &Model3D,
    support_polys: &[Vec<Vec2>],
) -> CachedGroundMap2D {
    let sc = m.scale.unwrap_or(1.0).max(0.05);
    let rot = m.transform.rotation;
    let hw = (m.width * sc * 0.5).max(30.0);
    let hd = (m.depth * sc * 0.5).max(30.0);
    let has_path = m.mountain_path.as_ref().is_some_and(|p| !p.is_empty());
    let is_water = m.ground_terrain_type.as_deref() == Some("water");
    let (min_lx, max_lx, min_ly, max_ly) = terrain_local_bounds(m);
    let (offsets, coords) = pack_support_polygons(support_polys);
    let clip = offsets.len() >= 2;

    let to_world = |lx: f64, ly: f64| -> Vec2 {
        let rp = rotate(Vec2 { x: lx, y: ly }, rot);
        Vec2 {
            x: m.transform.x + rp.x,
            y: m.transform.y + rp.y,
        }
    };

    const N: usize = 34;
    let dx = (max_lx - min_lx) / (N - 1) as f64;
    let dy = (max_ly - min_ly) / (N - 1) as f64;
    let mut cov_grid = vec![0.0_f64; N * N];

    for iy in 0..N {
        let ly = min_ly + iy as f64 * dy;
        for ix in 0..N {
            let lx = min_lx + ix as f64 * dx;
            let wp = to_world(lx, ly);
            let idx = iy * N + ix;
            if clip && !is_point_in_any_polygon(wp.x, wp.y, &offsets, &coords) {
                continue;
            }
            let nr = if has_path {
                evaluate_mountain_local(lx, ly, m).0
            } else {
                let raw_r = (lx / hw).hypot(ly / hd);
                if is_water {
                    let ang = ly.atan2(lx);
                    let wobble = 1.0 + 0.035 * (3.0 * ang).sin() + 0.02 * (5.0 * ang + 0.7).cos();
                    raw_r / wobble
                } else {
                    raw_r
                }
            };
            cov_grid[idx] = (1.0 - nr).max(0.0);
        }
    }

    let mut quads = Vec::with_capacity(512);
    for iy in 0..(N - 1) {
        let ly0 = min_ly + iy as f64 * dy;
        let ly1 = ly0 + dy;
        for ix in 0..(N - 1) {
            let c00 = cov_grid[iy * N + ix];
            let c10 = cov_grid[iy * N + ix + 1];
            let c11 = cov_grid[(iy + 1) * N + ix + 1];
            let c01 = cov_grid[(iy + 1) * N + ix];
            let c_avg = (c00 + c10 + c11 + c01) * 0.25;
            if c_avg < 0.015 {
                continue;
            }
            let band = if c_avg >= 0.54 {
                2
            } else if c_avg >= 0.26 {
                1
            } else {
                0
            };
            let lx0 = min_lx + ix as f64 * dx;
            let lx1 = lx0 + dx;
            let raw = [
                to_world(lx0, ly0),
                to_world(lx1, ly0),
                to_world(lx1, ly1),
                to_world(lx0, ly1),
            ];
            if let Some(clipped) = clip_quad_to_support(raw, support_polys, &offsets, &coords) {
                quads.push((clipped, band));
            }
        }
    }

    // Shoreline / perimeter iso-contour at cov == 0.02, clipped cleanly against table/plate
    let mut contour_segs = Vec::new();
    let iso = 0.02_f64;
    let interp = |a: f64, b: f64| -> f64 {
        let denom = b - a;
        if denom.abs() < 1e-9 { 0.5 } else { ((iso - a) / denom).clamp(0.0, 1.0) }
    };
    for iy in 0..(N - 1) {
        let ly0 = min_ly + iy as f64 * dy;
        let ly1 = ly0 + dy;
        for ix in 0..(N - 1) {
            let c00 = cov_grid[iy * N + ix];
            let c10 = cov_grid[iy * N + ix + 1];
            let c11 = cov_grid[(iy + 1) * N + ix + 1];
            let c01 = cov_grid[(iy + 1) * N + ix];
            let mask = ((c00 >= iso) as u8)
                | (((c10 >= iso) as u8) << 1)
                | (((c11 >= iso) as u8) << 2)
                | (((c01 >= iso) as u8) << 3);
            if mask == 0 || mask == 15 {
                continue;
            }
            let lx0 = min_lx + ix as f64 * dx;
            let lx1 = lx0 + dx;
            let e0 = (lx0 + interp(c00, c10) * dx, ly0);
            let e1 = (lx1, ly0 + interp(c10, c11) * dy);
            let e2 = (lx0 + interp(c01, c11) * dx, ly1);
            let e3 = (lx0, ly0 + interp(c00, c01) * dy);
            let pairs: &[((f64, f64), (f64, f64))] = match mask {
                1 | 14 => &[(e3, e0)],
                2 | 13 => &[(e0, e1)],
                3 | 12 => &[(e3, e1)],
                4 | 11 => &[(e1, e2)],
                5 => &[(e3, e2), (e0, e1)],
                6 | 9 => &[(e0, e2)],
                7 | 8 => &[(e3, e2)],
                10 => &[(e3, e0), (e1, e2)],
                _ => &[],
            };
            for &(a, b) in pairs {
                if let Some(seg) = clip_segment_to_support(to_world(a.0, a.1), to_world(b.0, b.1), &offsets, &coords) {
                    contour_segs.push(seg);
                }
            }
        }
    }

    // Subtle sinusoidal water ripples clipped to both water body and table/plate
    let mut ripple_segs = Vec::new();
    if is_water && !has_path {
        let wave_amp = (hd * 0.035).min(6.0);
        for &(ry, span_x, phase) in &[
            (-0.34, 0.42, 0.0_f64),
            (-0.11, 0.56, 1.2),
            (0.12, 0.52, 2.4),
            (0.34, 0.36, 0.7),
        ] {
            for s in 0..12 {
                let t0 = (s as f64 / 12.0) * 2.0 - 1.0;
                let t1 = ((s + 1) as f64 / 12.0) * 2.0 - 1.0;
                let p0 = to_world(
                    t0 * hw * span_x,
                    hd * ry + (t0 * std::f64::consts::TAU + phase).sin() * wave_amp,
                );
                let p1 = to_world(
                    t1 * hw * span_x,
                    hd * ry + (t1 * std::f64::consts::TAU + phase).sin() * wave_amp,
                );
                if let Some(seg) = clip_segment_to_support(p0, p1, &offsets, &coords) {
                    ripple_segs.push(seg);
                }
            }
        }
    }

    CachedGroundMap2D {
        quads,
        contour_segs,
        ripple_segs,
    }
}
