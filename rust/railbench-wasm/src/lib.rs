use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use wasm_bindgen::prelude::*;

#[derive(Copy, Clone)]
struct State {
    res: f64,
    dist_m: f64,
    node: usize,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.res.to_bits() == other.res.to_bits() && self.node == other.node
    }
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap by electrical resistance (tie-break by node index)
        other
            .res
            .total_cmp(&self.res)
            .then_with(|| other.node.cmp(&self.node))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Multi-source Dijkstra solver for electrical rail / catenary resistance networks.
///
/// Inputs are flat CSR arrays (zero JSON overhead):
/// - `adj_offsets`: length `node_count + 1`, CSR row offsets
/// - `adj_neighbors`: flat neighbor node indices
/// - `piece_len_m`: length `node_count`, physical length of each piece in meters
/// - `feeder_indices`: node indices that act as power feeders
/// - `rail_ohm_per_meter`: resistance per meter
/// - `joint_resistance_ohm`: resistance per rail joint
///
/// Returns a flat `Vec<f64>` of length `4 * node_count`:
/// - `[0 .. n)`: `min_resistance` (`f64::INFINITY` if unreachable)
/// - `[n .. 2*n)`: `min_distance` (`f64::INFINITY` if unreachable)
/// - `[2*n .. 3*n)`: `parent_index` (`-1.0` for feeder or unreachable)
/// - `[3*n .. 4*n)`: `edge_res_to_parent` (`0.0` if unreachable)
#[wasm_bindgen]
pub fn solve_power_dijkstra(
    node_count: usize,
    adj_offsets: &[u32],
    adj_neighbors: &[u32],
    piece_len_m: &[f64],
    feeder_indices: &[u32],
    rail_ohm_per_meter: f64,
    joint_resistance_ohm: f64,
) -> Vec<f64> {
    let mut out = vec![0.0_f64; node_count * 4];
    if node_count == 0 {
        return out;
    }

    let (min_res, rest) = out.split_at_mut(node_count);
    let (min_dist, rest) = rest.split_at_mut(node_count);
    let (parent, edge_res_to_parent) = rest.split_at_mut(node_count);

    for i in 0..node_count {
        min_res[i] = f64::INFINITY;
        min_dist[i] = f64::INFINITY;
        parent[i] = -1.0;
        edge_res_to_parent[i] = 0.0;
    }

    let mut heap = BinaryHeap::with_capacity(node_count);

    for &f_idx_u32 in feeder_indices {
        let f_idx = f_idx_u32 as usize;
        if f_idx >= node_count {
            continue;
        }
        let initial_res = (piece_len_m[f_idx] * 0.5) * rail_ohm_per_meter;
        if initial_res < min_res[f_idx] {
            min_res[f_idx] = initial_res;
            min_dist[f_idx] = 0.0;
            parent[f_idx] = -1.0;
            edge_res_to_parent[f_idx] = initial_res;
            heap.push(State {
                res: initial_res,
                dist_m: 0.0,
                node: f_idx,
            });
        }
    }

    while let Some(State { res, dist_m, node }) = heap.pop() {
        if res > min_res[node] {
            continue;
        }

        let start = adj_offsets.get(node).copied().unwrap_or(0) as usize;
        let end = adj_offsets.get(node + 1).copied().unwrap_or(start as u32) as usize;
        let end = end.min(adj_neighbors.len());

        for &nbr_u32 in &adj_neighbors[start..end] {
            let nbr = nbr_u32 as usize;
            if nbr >= node_count {
                continue;
            }
            let len_m = piece_len_m[nbr];
            let edge_res = len_m * rail_ohm_per_meter + joint_resistance_ohm;
            let new_res = res + edge_res;
            let new_dist = dist_m + len_m;

            if new_res < min_res[nbr] {
                min_res[nbr] = new_res;
                min_dist[nbr] = new_dist;
                parent[nbr] = node as f64;
                edge_res_to_parent[nbr] = edge_res;
                heap.push(State {
                    res: new_res,
                    dist_m: new_dist,
                    node: nbr,
                });
            }
        }
    }

    out
}

#[inline]
fn mountain_noise(nx: f64, nz: f64) -> f64 {
    let s1 = (nx * 3.1415 + 0.5).sin() * (nz * 3.1415 + 0.3).cos();
    let s2 = (nx * 6.28 + nz * 4.71).sin() * 0.35;
    let s3 = (nx * 9.42 - nz * 7.85).cos() * 0.15;
    s1 + s2 + s3
}

#[inline]
fn natural_mountain_height_at(nx: f64, nz: f64, height: f64) -> f64 {
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
///
/// `strokes` is an optional flat array of `[lx, lz, radius, stroke_height, ...]` (stride 4)
/// for sculpted brush-path mountains. When empty, evaluates the standard elliptical mountain profile.
#[wasm_bindgen]
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

#[inline]
fn norm_angle_deg(a: f64) -> f64 {
    let r = a % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

/// Spatial-hashed connector matcher for `updatePieceConnections`.
///
/// `connectors` has stride 6 per connector: `[x, y, angle_deg, height, piece_idx, conn_idx]`.
/// Returns `Vec<i32>` of length `2 * count`: `[matched_piece_idx, matched_conn_idx, ...]`
/// (`-1, -1` when unmatched).
#[wasm_bindgen]
pub fn match_connectors_spatial(
    connectors: &[f64],
    dist_tolerance: f64,
    angle_tolerance: f64,
    height_tolerance: f64,
) -> Vec<i32> {
    let count = connectors.len() / 6;
    let mut out = vec![-1_i32; count * 2];
    if count < 2 {
        return out;
    }

    let cell_size = (dist_tolerance * 4.0).max(16.0);
    let inv_cell = 1.0 / cell_size;
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::with_capacity(count);

    for i in 0..count {
        let base = i * 6;
        let gx = (connectors[base] * inv_cell).floor() as i32;
        let gy = (connectors[base + 1] * inv_cell).floor() as i32;
        grid.entry((gx, gy)).or_default().push(i);
    }

    let dist_tol_sq = dist_tolerance * dist_tolerance;

    for i in 0..count {
        let b1 = i * 6;
        let x1 = connectors[b1];
        let y1 = connectors[b1 + 1];
        let a1 = connectors[b1 + 2];
        let h1 = connectors[b1 + 3];
        let p1 = connectors[b1 + 4] as i32;

        let gx = (x1 * inv_cell).floor() as i32;
        let gy = (y1 * inv_cell).floor() as i32;

        'search: for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(bucket) = grid.get(&(gx + dx, gy + dy)) {
                    for &j in bucket {
                        let b2 = j * 6;
                        let p2 = connectors[b2 + 4] as i32;
                        if p2 == p1 {
                            continue;
                        }
                        if (h1 - connectors[b2 + 3]).abs() > height_tolerance {
                            continue;
                        }
                        let ddx = x1 - connectors[b2];
                        let ddy = y1 - connectors[b2 + 1];
                        if ddx * ddx + ddy * ddy > dist_tol_sq {
                            continue;
                        }
                        let diff_a = (norm_angle_deg(a1 - connectors[b2 + 2]) - 180.0).abs();
                        if diff_a <= angle_tolerance {
                            out[i * 2] = p2;
                            out[i * 2 + 1] = connectors[b2 + 5] as i32;
                            break 'search;
                        }
                    }
                }
            }
        }
    }

    out
}

#[inline]
fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn is_point_in_any_polygon(
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

/// Evaluates the full 3D ground-terrain vertex grid: support-polygon clipping,
/// organic height noise, building box-SDF flattening, and track/road corridor carving.
///
/// Returns flat `Vec<f32>` of stride 4 per vertex: `[height, in_bounds (0/1), is_water (0/1), flatten_weight]`.
#[wasm_bindgen]
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

/// Union-Find (DSU) short-circuit solver for 2-wire railway polarity checks.
///
/// - `edges`: stride 4 (`[n1L, n1R, n2L, n2R]`) per electrical joint
/// - `rail_pairs`: stride 2 (`[railA, railB]`) per piece/crossing check
///
/// Returns `Vec<u8>` of length `edge_count + pair_count`:
/// - first `edge_count` bytes: `1` if the joint triggers a short circuit
/// - next `pair_count` bytes: `1` if the rail pair ends up shorted
#[wasm_bindgen]
pub fn solve_short_circuit_dsu(
    terminal_count: usize,
    edges: &[u32],
    rail_pairs: &[u32],
) -> Vec<u8> {
    let edge_count = edges.len() / 4;
    let pair_count = rail_pairs.len() / 2;
    let mut out = vec![0_u8; edge_count + pair_count];
    if terminal_count == 0 {
        return out;
    }

    let mut parent: Vec<usize> = (0..terminal_count).collect();
    let mut rank = vec![0_u8; terminal_count];

    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            let gp = parent[parent[x]];
            parent[x] = gp;
            x = gp;
        }
        x
    }

    fn union(parent: &mut [usize], rank: &mut [u8], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra == rb {
            return;
        }
        if rank[ra] < rank[rb] {
            parent[ra] = rb;
        } else if rank[ra] > rank[rb] {
            parent[rb] = ra;
        } else {
            parent[rb] = ra;
            rank[ra] = rank[ra].saturating_add(1);
        }
    }

    for e in 0..edge_count {
        let b = e * 4;
        let n1l = edges[b] as usize;
        let n1r = edges[b + 1] as usize;
        let n2l = edges[b + 2] as usize;
        let n2r = edges[b + 3] as usize;
        if n1l >= terminal_count
            || n1r >= terminal_count
            || n2l >= terminal_count
            || n2r >= terminal_count
        {
            continue;
        }
        let r1l = find(&mut parent, n1l);
        let r1r = find(&mut parent, n1r);
        let r2l = find(&mut parent, n2l);
        let r2r = find(&mut parent, n2r);

        let will_short = r1l == r2l || r1r == r2r || r1l == r1r || r2l == r2r;
        if will_short && r1l != r2r {
            out[e] = 1;
        }
        union(&mut parent, &mut rank, n1l, n2r);
        union(&mut parent, &mut rank, n1r, n2l);
    }

    for p in 0..pair_count {
        let a = rail_pairs[p * 2] as usize;
        let b = rail_pairs[p * 2 + 1] as usize;
        if a < terminal_count && b < terminal_count && find(&mut parent, a) == find(&mut parent, b)
        {
            out[edge_count + p] = 1;
        }
    }

    out
}

/// Sweeps a 2D tunnel cross-section along a 3D centerline path into a flat `Float32Array` of vertex positions.
///
/// - `path_xyz`: stride 3 (`[x, y, z, ...]`)
/// - `section_xy`: stride 2 (`[x, y, ...]`)
#[wasm_bindgen]
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
#[wasm_bindgen]
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
            let denom = (if len1 == 0.0 { 1.0 } else { len1 }) * (if len2 == 0.0 { 1.0 } else { len2 });
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

/// Evaluates the cubic Hermite flextrack spline and returns flat `Vec<f64>` of stride 5 per sample:
/// `[x, y, nx, ny, tangent_angle_rad]`.
#[wasm_bindgen]
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

/// Computes per-piece start & end elevations along a ramp/helix chain.
///
/// - `piece_lengths`: length in mm of each piece in sequence
/// - `is_curve_left`: `1` if piece connects at connector 1 (left curve), `0` otherwise
///
/// Returns flat `Vec<f64>` of stride 2 per piece: `[start_elev, end_elev]` relative to `target_layer_base_h`.
#[wasm_bindgen]
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
///
/// - `track_samples`: stride 4 (`[wx, wy, wz, clearance_r]`)
/// - `candidates`: stride 3 (`[wx, wy, wz]`)
///
/// Returns `Vec<u8>` of length `candidates.len() / 3`: `1` if blocked by a nearby track, `0` if clear.
#[wasm_bindgen]
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

    for i in 0..n_cands {
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
                            blocked[i] = 1;
                            break 'outer;
                        }
                    }
                }
            }
        }
    }

    blocked
}

/// Evaluates whether batch world-space track centerline points (`pts_xye`, stride 3: `[wx, wy, elev]`)
/// lie inside an elliptical mountain footprint (`norm_r <= r_limit`) and below the mountain surface.
///
/// Returns `Vec<f32>` of stride 2 per point: `[norm_r, surface_height]`.
#[wasm_bindgen]
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
    fn dijkstra_linear_chain() {
        // 0 -- 1 -- 2, feeder at 0
        let offsets = [0u32, 1, 3, 4];
        let neighbors = [1u32, 0, 2, 1];
        let lens = [0.2_f64, 0.2, 0.2];
        let feeders = [0u32];
        let out = solve_power_dijkstra(3, &offsets, &neighbors, &lens, &feeders, 0.1, 0.01);
        assert!((out[0] - 0.01).abs() < 1e-9);
        assert!((out[1] - 0.04).abs() < 1e-9);
        assert!((out[2] - 0.07).abs() < 1e-9);
        assert_eq!(out[6], -1.0);
        assert_eq!(out[7], 0.0);
        assert_eq!(out[8], 1.0);
    }

    #[test]
    fn mountain_heightfield_center_peak() {
        let h = evaluate_mountain_heightfield(4, 4, 200.0, 200.0, 150.0, &[]);
        assert_eq!(h.len(), 25);
        // Center (2, 2) should be positive; corners (0, 0) are outside unit circle (r = sqrt(2) >= 1) => 0
        assert!(h[2 * 5 + 2] > 50.0);
        assert_eq!(h[0], 0.0);
    }

    #[test]
    fn spatial_connector_matching() {
        // Piece 0 conn 0 at (100, 0, angle 0) <-> Piece 1 conn 0 at (101, 0, angle 180)
        let conns = [
            100.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            101.0, 0.0, 180.0, 0.0, 1.0, 0.0,
        ];
        let matched = match_connectors_spatial(&conns, 4.0, 35.0, 1.0);
        assert_eq!(matched, vec![1, 0, 0, 0]);
    }
}
