/* tslint:disable */
/* eslint-disable */

/**
 * Sweeps a 2D tunnel cross-section along a 3D centerline path into a flat `Float32Array` of vertex positions.
 *
 * - `path_xyz`: stride 3 (`[x, y, z, ...]`)
 * - `section_xy`: stride 2 (`[x, y, ...]`)
 */
export function build_tunnel_sweep_positions(path_xyz: Float32Array | number[], section_xy: Float32Array | number[]): Float32Array;

/**
 * Evaluates the full 3D ground-terrain vertex grid: support-polygon clipping,
 * organic height noise, building box-SDF flattening, and track/road corridor carving.
 *
 * Returns flat `Vec<f32>` of stride 4 per vertex: `[height, in_bounds (0/1), is_water (0/1), flatten_weight]`.
 */
export function carve_ground_terrain_grid(segs_x: number, segs_z: number, width: number, depth: number, max_height: number, base_layer_elev: number, model_x: number, model_z: number, cos_m: number, sin_m: number, transition_dist: number, is_water: boolean, r_dist_grid: Float32Array | number[], support_poly_offsets: Uint32Array | number[], support_poly_coords: Float32Array | number[], buildings: Float32Array | number[], corridor_meta: Float32Array | number[], corridor_pts: Float32Array | number[]): Float32Array;

/**
 * Evaluates a 2D procedural mountain heightfield on a `(nx_steps + 1) x (nz_steps + 1)` grid.
 *
 * `strokes` is an optional flat array of `[lx, lz, radius, stroke_height, ...]` (stride 4)
 * for sculpted brush-path mountains. When empty, evaluates the standard elliptical mountain profile.
 */
export function evaluate_mountain_heightfield(nx_steps: number, nz_steps: number, half_w: number, half_d: number, height: number, strokes: Float32Array | number[]): Float32Array;

/**
 * Spatial-hashed connector matcher for `updatePieceConnections`.
 *
 * `connectors` has stride 6 per connector: `[x, y, angle_deg, height, piece_idx, conn_idx]`.
 * Returns `Vec<i32>` of length `2 * count`: `[matched_piece_idx, matched_conn_idx, ...]`
 * (`-1, -1` when unmatched).
 */
export function match_connectors_spatial(connectors: Float64Array | number[], dist_tolerance: number, angle_tolerance: number, height_tolerance: number): Int32Array;

/**
 * Clamps, rounds (0.1 mm), deduplicates micro-segments, and removes collinear points
 * from a 2D table-cutout polygon (`raw_xy` stride 2).
 */
export function simplify_cutout_polygon(raw_xy: Float64Array | number[], min_x: number, max_x: number, min_y: number, max_y: number, dedup_dist: number, collinear_tol: number): Float64Array;

/**
 * Multi-source Dijkstra solver for electrical rail / catenary resistance networks.
 *
 * Inputs are flat CSR arrays (zero JSON overhead):
 * - `adj_offsets`: length `node_count + 1`, CSR row offsets
 * - `adj_neighbors`: flat neighbor node indices
 * - `piece_len_m`: length `node_count`, physical length of each piece in meters
 * - `feeder_indices`: node indices that act as power feeders
 * - `rail_ohm_per_meter`: resistance per meter
 * - `joint_resistance_ohm`: resistance per rail joint
 *
 * Returns a flat `Vec<f64>` of length `4 * node_count`:
 * - `[0 .. n)`: `min_resistance` (`f64::INFINITY` if unreachable)
 * - `[n .. 2*n)`: `min_distance` (`f64::INFINITY` if unreachable)
 * - `[2*n .. 3*n)`: `parent_index` (`-1.0` for feeder or unreachable)
 * - `[3*n .. 4*n)`: `edge_res_to_parent` (`0.0` if unreachable)
 */
export function solve_power_dijkstra(node_count: number, adj_offsets: Uint32Array | number[], adj_neighbors: Uint32Array | number[], piece_len_m: Float64Array | number[], feeder_indices: Uint32Array | number[], rail_ohm_per_meter: number, joint_resistance_ohm: number): Float64Array;

/**
 * Union-Find (DSU) short-circuit solver for 2-wire railway polarity checks.
 *
 * - `edges`: stride 4 (`[n1L, n1R, n2L, n2R]`) per electrical joint
 * - `rail_pairs`: stride 2 (`[railA, railB]`) per piece/crossing check
 *
 * Returns `Vec<u8>` of length `edge_count + pair_count`:
 * - first `edge_count` bytes: `1` if the joint triggers a short circuit
 * - next `pair_count` bytes: `1` if the rail pair ends up shorted
 */
export function solve_short_circuit_dsu(terminal_count: number, edges: Uint32Array | number[], rail_pairs: Uint32Array | number[]): Uint8Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly build_tunnel_sweep_positions: (a: number, b: number, c: number, d: number) => [number, number];
    readonly carve_ground_terrain_grid: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number, m: number, n: number, o: number, p: number, q: number, r: number, s: number, t: number, u: number, v: number, w: number, x: number) => [number, number];
    readonly evaluate_mountain_heightfield: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly match_connectors_spatial: (a: number, b: number, c: number, d: number, e: number) => [number, number];
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
