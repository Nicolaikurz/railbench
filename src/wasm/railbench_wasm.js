/* @ts-self-types="./railbench_wasm.d.ts" */

/**
 * @param {Float32Array | number[]} path_xyz
 * @param {Float32Array | number[]} section_xy
 * @returns {Float32Array}
 */
export function build_tunnel_sweep_positions(path_xyz, section_xy) {
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passArrayF32ToWasm0(path_xyz, wasm.__wbindgen_malloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passArrayF32ToWasm0(section_xy, wasm.__wbindgen_malloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 4, 4); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 4, 4); }
        throw e;
    }
    const ret = wasm.build_tunnel_sweep_positions(ptr0, len0, ptr1, len1);
    var v3 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v3;
}

/**
 * @param {number} segs_x
 * @param {number} segs_z
 * @param {number} width
 * @param {number} depth
 * @param {number} max_height
 * @param {number} base_layer_elev
 * @param {number} model_x
 * @param {number} model_z
 * @param {number} cos_m
 * @param {number} sin_m
 * @param {number} transition_dist
 * @param {boolean} is_water
 * @param {Float32Array | number[]} r_dist_grid
 * @param {Uint32Array | number[]} support_poly_offsets
 * @param {Float32Array | number[]} support_poly_coords
 * @param {Float32Array | number[]} buildings
 * @param {Float32Array | number[]} corridor_meta
 * @param {Float32Array | number[]} corridor_pts
 * @returns {Float32Array}
 */
export function carve_ground_terrain_grid(segs_x, segs_z, width, depth, max_height, base_layer_elev, model_x, model_z, cos_m, sin_m, transition_dist, is_water, r_dist_grid, support_poly_offsets, support_poly_coords, buildings, corridor_meta, corridor_pts) {
    const val7 = +segs_x;
    const val8 = +segs_z;
    const val9 = +width;
    const val10 = +depth;
    const val11 = +max_height;
    const val12 = +base_layer_elev;
    const val13 = +model_x;
    const val14 = +model_z;
    const val15 = +cos_m;
    const val16 = +sin_m;
    const val17 = +transition_dist;
    const val18 = +is_water;
    let ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5;
    try {
        ptr0 = passArrayF32ToWasm0(r_dist_grid, wasm.__wbindgen_malloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passArray32ToWasm0(support_poly_offsets, wasm.__wbindgen_malloc);
        len1 = WASM_VECTOR_LEN;
        ptr2 = passArrayF32ToWasm0(support_poly_coords, wasm.__wbindgen_malloc);
        len2 = WASM_VECTOR_LEN;
        ptr3 = passArrayF32ToWasm0(buildings, wasm.__wbindgen_malloc);
        len3 = WASM_VECTOR_LEN;
        ptr4 = passArrayF32ToWasm0(corridor_meta, wasm.__wbindgen_malloc);
        len4 = WASM_VECTOR_LEN;
        ptr5 = passArrayF32ToWasm0(corridor_pts, wasm.__wbindgen_malloc);
        len5 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr5) { wasm.__wbindgen_free(ptr5, len5 * 4, 4); }
        if (ptr4) { wasm.__wbindgen_free(ptr4, len4 * 4, 4); }
        if (ptr3) { wasm.__wbindgen_free(ptr3, len3 * 4, 4); }
        if (ptr2) { wasm.__wbindgen_free(ptr2, len2 * 4, 4); }
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 4, 4); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 4, 4); }
        throw e;
    }
    const ret = wasm.carve_ground_terrain_grid(val7, val8, val9, val10, val11, val12, val13, val14, val15, val16, val17, val18, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4, ptr5, len5);
    var v19 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v19;
}

/**
 * @param {Float32Array | number[]} track_samples
 * @param {Float32Array | number[]} candidates
 * @param {number} vertical_tol
 * @returns {Uint8Array}
 */
export function check_catenary_mast_clearances(track_samples, candidates, vertical_tol) {
    const val3 = +vertical_tol;
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passArrayF32ToWasm0(track_samples, wasm.__wbindgen_malloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passArrayF32ToWasm0(candidates, wasm.__wbindgen_malloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 4, 4); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 4, 4); }
        throw e;
    }
    const ret = wasm.check_catenary_mast_clearances(ptr0, len0, ptr1, len1, val3);
    var v4 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v4;
}

/**
 * @param {Float64Array | number[]} piece_lengths
 * @param {Uint8Array | number[]} is_curve_left
 * @param {number} h_start
 * @param {number} total_rise
 * @param {number} target_layer_base_h
 * @returns {Float64Array}
 */
export function compute_ramp_chain_elevations(piece_lengths, is_curve_left, h_start, total_rise, target_layer_base_h) {
    const val3 = +h_start;
    const val4 = +total_rise;
    const val5 = +target_layer_base_h;
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passArrayF64ToWasm0(piece_lengths, wasm.__wbindgen_malloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passArray8ToWasm0(is_curve_left, wasm.__wbindgen_malloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 8, 8); }
        throw e;
    }
    const ret = wasm.compute_ramp_chain_elevations(ptr0, len0, ptr1, len1, val3, val4, val5);
    var v6 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
    return v6;
}

/**
 * @param {number} nx_steps
 * @param {number} nz_steps
 * @param {number} half_w
 * @param {number} half_d
 * @param {number} height
 * @param {Float32Array | number[]} strokes
 * @returns {Float32Array}
 */
export function evaluate_mountain_heightfield(nx_steps, nz_steps, half_w, half_d, height, strokes) {
    const val2 = +nx_steps;
    const val3 = +nz_steps;
    const val4 = +half_w;
    const val5 = +half_d;
    const val6 = +height;
    const ptr0 = passArrayF32ToWasm0(strokes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.evaluate_mountain_heightfield(val2, val3, val4, val5, val6, ptr0, len0);
    var v7 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v7;
}

/**
 * @param {Float32Array | number[]} pts_xye
 * @param {number} m_x
 * @param {number} m_z
 * @param {number} cos_inv
 * @param {number} sin_inv
 * @param {number} half_w
 * @param {number} half_d
 * @param {number} height
 * @returns {Float32Array}
 */
export function evaluate_mountain_track_samples(pts_xye, m_x, m_z, cos_inv, sin_inv, half_w, half_d, height) {
    const val2 = +m_x;
    const val3 = +m_z;
    const val4 = +cos_inv;
    const val5 = +sin_inv;
    const val6 = +half_w;
    const val7 = +half_d;
    const val8 = +height;
    const ptr0 = passArrayF32ToWasm0(pts_xye, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.evaluate_mountain_track_samples(ptr0, len0, val2, val3, val4, val5, val6, val7, val8);
    var v9 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v9;
}

/**
 * @param {Float64Array | number[]} connectors
 * @param {number} dist_tolerance
 * @param {number} angle_tolerance
 * @param {number} height_tolerance
 * @returns {Int32Array}
 */
export function match_connectors_spatial(connectors, dist_tolerance, angle_tolerance, height_tolerance) {
    const val2 = +dist_tolerance;
    const val3 = +angle_tolerance;
    const val4 = +height_tolerance;
    const ptr0 = passArrayF64ToWasm0(connectors, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.match_connectors_spatial(ptr0, len0, val2, val3, val4);
    var v5 = getArrayI32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v5;
}

/**
 * @param {number} chord_dist
 * @param {number} h1
 * @param {number} h2
 * @param {number} h3
 * @param {number} num_samples
 * @returns {Float64Array}
 */
export function sample_flex_spline_points(chord_dist, h1, h2, h3, num_samples) {
    const ret = wasm.sample_flex_spline_points(chord_dist, h1, h2, h3, num_samples);
    var v1 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
    return v1;
}

/**
 * @param {Float64Array | number[]} raw_xy
 * @param {number} min_x
 * @param {number} max_x
 * @param {number} min_y
 * @param {number} max_y
 * @param {number} dedup_dist
 * @param {number} collinear_tol
 * @returns {Float64Array}
 */
export function simplify_cutout_polygon(raw_xy, min_x, max_x, min_y, max_y, dedup_dist, collinear_tol) {
    const val2 = +min_x;
    const val3 = +max_x;
    const val4 = +min_y;
    const val5 = +max_y;
    const val6 = +dedup_dist;
    const val7 = +collinear_tol;
    const ptr0 = passArrayF64ToWasm0(raw_xy, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.simplify_cutout_polygon(ptr0, len0, val2, val3, val4, val5, val6, val7);
    var v8 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
    return v8;
}

/**
 * @param {number} node_count
 * @param {Uint32Array | number[]} adj_offsets
 * @param {Uint32Array | number[]} adj_neighbors
 * @param {Float64Array | number[]} piece_len_m
 * @param {Uint32Array | number[]} feeder_indices
 * @param {number} rail_ohm_per_meter
 * @param {number} joint_resistance_ohm
 * @returns {Float64Array}
 */
export function solve_power_dijkstra(node_count, adj_offsets, adj_neighbors, piece_len_m, feeder_indices, rail_ohm_per_meter, joint_resistance_ohm) {
    const val5 = +node_count;
    const val6 = +rail_ohm_per_meter;
    const val7 = +joint_resistance_ohm;
    let ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3;
    try {
        ptr0 = passArray32ToWasm0(adj_offsets, wasm.__wbindgen_malloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passArray32ToWasm0(adj_neighbors, wasm.__wbindgen_malloc);
        len1 = WASM_VECTOR_LEN;
        ptr2 = passArrayF64ToWasm0(piece_len_m, wasm.__wbindgen_malloc);
        len2 = WASM_VECTOR_LEN;
        ptr3 = passArray32ToWasm0(feeder_indices, wasm.__wbindgen_malloc);
        len3 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr3) { wasm.__wbindgen_free(ptr3, len3 * 4, 4); }
        if (ptr2) { wasm.__wbindgen_free(ptr2, len2 * 8, 8); }
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 4, 4); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 4, 4); }
        throw e;
    }
    const ret = wasm.solve_power_dijkstra(val5, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, val6, val7);
    var v8 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
    return v8;
}

/**
 * @param {number} terminal_count
 * @param {Uint32Array | number[]} edges
 * @param {Uint32Array | number[]} rail_pairs
 * @returns {Uint8Array}
 */
export function solve_short_circuit_dsu(terminal_count, edges, rail_pairs) {
    const val3 = +terminal_count;
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passArray32ToWasm0(edges, wasm.__wbindgen_malloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passArray32ToWasm0(rail_pairs, wasm.__wbindgen_malloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 4, 4); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 4, 4); }
        throw e;
    }
    const ret = wasm.solve_short_circuit_dsu(val3, ptr0, len0, ptr1, len1);
    var v4 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v4;
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            table.set(0, undefined);
            table.set(1024, undefined);
            table.set(1025, null);
            table.set(1026, true);
            table.set(1027, false);
        },
    };
    return {
        __proto__: null,
        "./railbench_wasm_bg.js": import0,
    };
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayI32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

let cachedInt32ArrayMemory0 = null;
function getInt32ArrayMemory0() {
    if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
        cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32ArrayMemory0;
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getUint32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArrayF32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getFloat32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArrayF64ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 8, 8) >>> 0;
    getFloat64ArrayMemory0().set(arg, ptr / 8);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedFloat32ArrayMemory0 = null;
    cachedFloat64ArrayMemory0 = null;
    cachedInt32ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

let __wbg_init_promise;
async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;
    if (__wbg_init_promise === undefined) {
        __wbg_init_promise = __wbg_init_once(module_or_path);
        __wbg_init_promise.catch(() => { __wbg_init_promise = undefined; });
    }
    return __wbg_init_promise;
}

async function __wbg_init_once(module_or_path) {

    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('railbench_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
