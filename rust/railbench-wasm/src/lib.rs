use wasm_bindgen::prelude::*;

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
    railbench_core::simulation::solve_power_dijkstra(
        node_count,
        adj_offsets,
        adj_neighbors,
        piece_len_m,
        feeder_indices,
        rail_ohm_per_meter,
        joint_resistance_ohm,
    )
}

#[wasm_bindgen]
pub fn solve_short_circuit_dsu(
    terminal_count: usize,
    edges: &[u32],
    rail_pairs: &[u32],
) -> Vec<u8> {
    railbench_core::simulation::solve_short_circuit_dsu(terminal_count, edges, rail_pairs)
}

#[wasm_bindgen]
pub fn evaluate_mountain_heightfield(
    nx_steps: usize,
    nz_steps: usize,
    half_w: f64,
    half_d: f64,
    height: f64,
    strokes: &[f32],
) -> Vec<f32> {
    railbench_core::terrain::evaluate_mountain_heightfield(
        nx_steps, nz_steps, half_w, half_d, height, strokes,
    )
}

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
    railbench_core::terrain::carve_ground_terrain_grid(
        segs_x,
        segs_z,
        width,
        depth,
        max_height,
        base_layer_elev,
        model_x,
        model_z,
        cos_m,
        sin_m,
        transition_dist,
        is_water,
        r_dist_grid,
        support_poly_offsets,
        support_poly_coords,
        buildings,
        corridor_meta,
        corridor_pts,
    )
}

#[wasm_bindgen]
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
    railbench_core::terrain::evaluate_mountain_track_samples(
        pts_xye, m_x, m_z, cos_inv, sin_inv, half_w, half_d, height,
    )
}

#[wasm_bindgen]
pub fn match_connectors_spatial(
    connectors: &[f64],
    dist_tolerance: f64,
    angle_tolerance: f64,
    height_tolerance: f64,
) -> Vec<i32> {
    railbench_core::snapping::match_connectors_spatial(
        connectors,
        dist_tolerance,
        angle_tolerance,
        height_tolerance,
    )
}

#[wasm_bindgen]
pub fn build_tunnel_sweep_positions(path_xyz: &[f32], section_xy: &[f32]) -> Vec<f32> {
    railbench_core::geometry::build_tunnel_sweep_positions(path_xyz, section_xy)
}

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
    railbench_core::geometry::simplify_cutout_polygon(
        raw_xy,
        min_x,
        max_x,
        min_y,
        max_y,
        dedup_dist,
        collinear_tol,
    )
}

#[wasm_bindgen]
pub fn sample_flex_spline_points(
    chord_dist: f64,
    h1: f64,
    h2: f64,
    h3: f64,
    num_samples: usize,
) -> Vec<f64> {
    railbench_core::geometry::sample_flex_spline_points(chord_dist, h1, h2, h3, num_samples)
}

#[wasm_bindgen]
pub fn compute_ramp_chain_elevations(
    piece_lengths: &[f64],
    is_curve_left: &[u8],
    h_start: f64,
    total_rise: f64,
    target_layer_base_h: f64,
) -> Vec<f64> {
    railbench_core::ramp::compute_ramp_chain_elevations(
        piece_lengths,
        is_curve_left,
        h_start,
        total_rise,
        target_layer_base_h,
    )
}

#[wasm_bindgen]
pub fn check_catenary_mast_clearances(
    track_samples: &[f32],
    candidates: &[f32],
    vertical_tol: f32,
) -> Vec<u8> {
    railbench_core::ramp::check_catenary_mast_clearances(track_samples, candidates, vertical_tol)
}
