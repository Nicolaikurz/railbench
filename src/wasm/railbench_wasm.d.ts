/* tslint:disable */
/* eslint-disable */

export function build_tunnel_sweep_positions(path_xyz: Float32Array | number[], section_xy: Float32Array | number[]): Float32Array;

export function carve_ground_terrain_grid(segs_x: number, segs_z: number, width: number, depth: number, max_height: number, base_layer_elev: number, model_x: number, model_z: number, cos_m: number, sin_m: number, transition_dist: number, is_water: boolean, r_dist_grid: Float32Array | number[], support_poly_offsets: Uint32Array | number[], support_poly_coords: Float32Array | number[], buildings: Float32Array | number[], corridor_meta: Float32Array | number[], corridor_pts: Float32Array | number[]): Float32Array;

export function check_catenary_mast_clearances(track_samples: Float32Array | number[], candidates: Float32Array | number[], vertical_tol: number): Uint8Array;

export function compute_ramp_chain_elevations(piece_lengths: Float64Array | number[], is_curve_left: Uint8Array | number[], h_start: number, total_rise: number, target_layer_base_h: number): Float64Array;

export function evaluate_mountain_heightfield(nx_steps: number, nz_steps: number, half_w: number, half_d: number, height: number, strokes: Float32Array | number[]): Float32Array;

export function evaluate_mountain_track_samples(pts_xye: Float32Array | number[], m_x: number, m_z: number, cos_inv: number, sin_inv: number, half_w: number, half_d: number, height: number): Float32Array;

export function match_connectors_spatial(connectors: Float64Array | number[], dist_tolerance: number, angle_tolerance: number, height_tolerance: number): Int32Array;

export function sample_flex_spline_points(chord_dist: number, h1: number, h2: number, h3: number, num_samples: number): Float64Array;

export function simplify_cutout_polygon(raw_xy: Float64Array | number[], min_x: number, max_x: number, min_y: number, max_y: number, dedup_dist: number, collinear_tol: number): Float64Array;

export function solve_power_dijkstra(node_count: number, adj_offsets: Uint32Array | number[], adj_neighbors: Uint32Array | number[], piece_len_m: Float64Array | number[], feeder_indices: Uint32Array | number[], rail_ohm_per_meter: number, joint_resistance_ohm: number): Float64Array;

export function solve_short_circuit_dsu(terminal_count: number, edges: Uint32Array | number[], rail_pairs: Uint32Array | number[]): Uint8Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly build_tunnel_sweep_positions: (a: number, b: number, c: number, d: number) => [number, number];
    readonly carve_ground_terrain_grid: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: number, t: number, u: number, v: number, w: number, x: number) => [number, number];
    readonly check_catenary_mast_clearances: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly compute_ramp_chain_elevations: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly evaluate_mountain_heightfield: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly evaluate_mountain_track_samples: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number];
    readonly match_connectors_spatial: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly sample_flex_spline_points: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly simplify_cutout_polygon: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly solve_power_dijkstra: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number];
    readonly solve_short_circuit_dsu: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
