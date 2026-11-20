/**
 * Optionaler Rust-WebAssembly-Beschleuniger für rechenintensive Kerne:
 *  - Multi-Source-Dijkstra für Fahrstrom-/Spannungsabfall-Simulation
 *  - Prozedurale 3D-Berg-/Gelände-Höhenfeld-Auswertung
 *
 * Fällt automatisch und transparent auf optimiertes TypeScript zurück,
 * falls das WASM-Artefakt (`npm run build:wasm`) noch nicht gebaut wurde.
 */

interface RailbenchWasmModule {
  default?: (input?: unknown) => Promise<unknown>;
  solve_power_dijkstra: (
    nodeCount: number,
    adjOffsets: Uint32Array,
    adjNeighbors: Uint32Array,
    pieceLenM: Float64Array,
    feederIndices: Uint32Array,
    railOhmPerMeter: number,
    jointResistanceOhm: number,
  ) => Float64Array;
  evaluate_mountain_heightfield: (
    nxSteps: number,
    nzSteps: number,
    halfW: number,
    halfD: number,
    height: number,
    strokes: Float32Array,
  ) => Float32Array;
  match_connectors_spatial?: (
    connectors: Float64Array,
    distTolerance: number,
    angleTolerance: number,
    heightTolerance: number,
  ) => Int32Array;
  carve_ground_terrain_grid?: (
    segsX: number,
    segsZ: number,
    width: number,
    depth: number,
    maxHeight: number,
    baseLayerElev: number,
    modelX: number,
    modelZ: number,
    cosM: number,
    sinM: number,
    transitionDist: number,
    isWater: boolean,
    rDistGrid: Float32Array,
    supportPolyOffsets: Uint32Array,
    supportPolyCoords: Float32Array,
    buildings: Float32Array,
    corridorMeta: Float32Array,
    corridorPts: Float32Array,
  ) => Float32Array;
  solve_short_circuit_dsu?: (
    terminalCount: number,
    edges: Uint32Array,
    railPairs: Uint32Array,
  ) => Uint8Array;
  build_tunnel_sweep_positions?: (
    pathXyz: Float32Array,
    sectionXy: Float32Array,
  ) => Float32Array;
  simplify_cutout_polygon?: (
    rawXy: Float64Array,
    minX: number,
    maxX: number,
    minY: number,
    maxY: number,
    dedupDist: number,
    collinearTol: number,
  ) => Float64Array;
  sample_flex_spline_points?: (
    chordDist: number,
    h1: number,
    h2: number,
    h3: number,
    numSamples: number,
  ) => Float64Array;
  compute_ramp_chain_elevations?: (
    pieceLengths: Float64Array,
    isCurveLeft: Uint8Array,
    hStart: number,
    totalRise: number,
    targetLayerBaseH: number,
  ) => Float64Array;
  check_catenary_mast_clearances?: (
    trackSamples: Float32Array,
    candidates: Float32Array,
    verticalTol: number,
  ) => Uint8Array;
  evaluate_mountain_track_samples?: (
    ptsXye: Float32Array,
    mX: number,
    mZ: number,
    cosInv: number,
    sinInv: number,
    halfW: number,
    halfD: number,
    height: number,
  ) => Float32Array;
}

let wasmMod: RailbenchWasmModule | null = null;
let wasmInitPromise: Promise<boolean> | null = null;

/** Gibt zurück, ob der Rust-WASM-Kernel aktiv geladen ist. */
export function isWasmReady(): boolean {
  return wasmMod !== null;
}

/**
 * Lädt den kompilierten Rust-WASM-Kernel (`src/wasm/railbench_wasm.js`), falls vorhanden.
 * Schlägt niemals fehl — bei fehlendem Build bleibt der JS-Fallback aktiv.
 */
export function initWasmKernel(): Promise<boolean> {
  if (wasmMod) return Promise.resolve(true);
  if (wasmInitPromise) return wasmInitPromise;

  wasmInitPromise = (async () => {
    try {
      // Dynamischer Pfad verhindert Vite-Build-Fehler, falls `npm run build:wasm` noch nicht lief
      const wasmModules = import.meta.glob("../wasm/railbench_wasm.js");
      const loader = wasmModules["../wasm/railbench_wasm.js"];
      if (!loader) return false;
      const mod = (await loader()) as unknown as RailbenchWasmModule;
      if (typeof mod.default === "function") {
        await mod.default();
      }
      if (
        typeof mod.solve_power_dijkstra === "function" &&
        typeof mod.evaluate_mountain_heightfield === "function"
      ) {
        wasmMod = mod;
        return true;
      }
    } catch {
      // Fallback bleibt aktiv
    }
    return false;
  })();

  return wasmInitPromise;
}

export interface DijkstraResult {
  minResistance: Float64Array;
  minDistance: Float64Array;
  parent: Int32Array;
  edgeResToParent: Float64Array;
}

/**
 * Multi-Source-Dijkstra auf einem CSR-Graphen.
 * Nutzt den Rust-WASM-Kernel wenn verfügbar, sonst einen schnellen binären Min-Heap in JS (O((V+E) log V)).
 */
export function solvePowerDijkstraKernel(
  nodeCount: number,
  adjOffsets: Uint32Array,
  adjNeighbors: Uint32Array,
  pieceLenM: Float64Array,
  feederIndices: Uint32Array,
  railOhmPerMeter: number,
  jointResistanceOhm: number,
): DijkstraResult {
  if (wasmMod && nodeCount > 0) {
    const packed = wasmMod.solve_power_dijkstra(
      nodeCount,
      adjOffsets,
      adjNeighbors,
      pieceLenM,
      feederIndices,
      railOhmPerMeter,
      jointResistanceOhm,
    );
    const minResistance = packed.slice(0, nodeCount);
    const minDistance = packed.slice(nodeCount, nodeCount * 2);
    const parentF64 = packed.subarray(nodeCount * 2, nodeCount * 3);
    const parent = new Int32Array(nodeCount);
    for (let i = 0; i < nodeCount; i++) parent[i] = parentF64[i]! | 0;
    const edgeResToParent = packed.slice(nodeCount * 3, nodeCount * 4);
    return { minResistance, minDistance, parent, edgeResToParent };
  }

  // Schneller JS-Fallback mit binärem Min-Heap (ersetzt langsames pq.sort + pieces.find)
  const minResistance = new Float64Array(nodeCount);
  const minDistance = new Float64Array(nodeCount);
  const parent = new Int32Array(nodeCount);
  const edgeResToParent = new Float64Array(nodeCount);

  minResistance.fill(Infinity);
  minDistance.fill(Infinity);
  parent.fill(-1);

  const heapNode: number[] = [];
  const heapRes: number[] = [];
  const heapDist: number[] = [];

  const pushHeap = (node: number, res: number, distM: number) => {
    let i = heapNode.length;
    heapNode.push(node);
    heapRes.push(res);
    heapDist.push(distM);
    while (i > 0) {
      const p = (i - 1) >> 1;
      if (heapRes[p]! <= heapRes[i]!) break;
      const tn = heapNode[p]!; heapNode[p] = heapNode[i]!; heapNode[i] = tn;
      const tr = heapRes[p]!; heapRes[p] = heapRes[i]!; heapRes[i] = tr;
      const td = heapDist[p]!; heapDist[p] = heapDist[i]!; heapDist[i] = td;
      i = p;
    }
  };

  const popHeap = (): [number, number, number] => {
    const outNode = heapNode[0]!;
    const outRes = heapRes[0]!;
    const outDist = heapDist[0]!;
    const lastN = heapNode.pop()!;
    const lastR = heapRes.pop()!;
    const lastD = heapDist.pop()!;
    if (heapNode.length > 0) {
      heapNode[0] = lastN;
      heapRes[0] = lastR;
      heapDist[0] = lastD;
      let i = 0;
      const n = heapNode.length;
      while (true) {
        const l = (i << 1) + 1;
        const r = l + 1;
        let smallest = i;
        if (l < n && heapRes[l]! < heapRes[smallest]!) smallest = l;
        if (r < n && heapRes[r]! < heapRes[smallest]!) smallest = r;
        if (smallest === i) break;
        const tn = heapNode[i]!; heapNode[i] = heapNode[smallest]!; heapNode[smallest] = tn;
        const tr = heapRes[i]!; heapRes[i] = heapRes[smallest]!; heapRes[smallest] = tr;
        const td = heapDist[i]!; heapDist[i] = heapDist[smallest]!; heapDist[smallest] = td;
        i = smallest;
      }
    }
    return [outNode, outRes, outDist];
  };

  for (let i = 0; i < feederIndices.length; i++) {
    const fIdx = feederIndices[i]!;
    if (fIdx >= nodeCount) continue;
    const initialRes = (pieceLenM[fIdx]! * 0.5) * railOhmPerMeter;
    if (initialRes < minResistance[fIdx]!) {
      minResistance[fIdx] = initialRes;
      minDistance[fIdx] = 0;
      parent[fIdx] = -1;
      edgeResToParent[fIdx] = initialRes;
      pushHeap(fIdx, initialRes, 0);
    }
  }

  while (heapNode.length > 0) {
    const [node, res, distM] = popHeap();
    if (res > minResistance[node]!) continue;
    const start = adjOffsets[node]!;
    const end = adjOffsets[node + 1]!;
    for (let e = start; e < end; e++) {
      const nbr = adjNeighbors[e]!;
      const lenM = pieceLenM[nbr]!;
      const edgeRes = lenM * railOhmPerMeter + jointResistanceOhm;
      const newRes = res + edgeRes;
      const newDist = distM + lenM;
      if (newRes < minResistance[nbr]!) {
        minResistance[nbr] = newRes;
        minDistance[nbr] = newDist;
        parent[nbr] = node;
        edgeResToParent[nbr] = edgeRes;
        pushHeap(nbr, newRes, newDist);
      }
    }
  }

  return { minResistance, minDistance, parent, edgeResToParent };
}

/**
 * Wertet ein prozedurales 3D-Berg-Höhenfeld aus (Rust-WASM wenn geladen, sonst JS).
 */
export function evaluateMountainHeightfieldKernel(
  nxSteps: number,
  nzSteps: number,
  halfW: number,
  halfD: number,
  height: number,
  strokes: Float32Array = new Float32Array(0),
): Float32Array {
  if (wasmMod) {
    return wasmMod.evaluate_mountain_heightfield(nxSteps, nzSteps, halfW, halfD, height, strokes);
  }

  const cols = nxSteps + 1;
  const rows = nzSteps + 1;
  const heights = new Float32Array(cols * rows);
  if (halfW <= 0 || halfD <= 0 || height <= 0) return heights;

  const hasStrokes = strokes.length >= 4;
  const invNx = nxSteps > 0 ? 1 / nxSteps : 0;
  const invNz = nzSteps > 0 ? 1 / nzSteps : 0;

  for (let iz = 0; iz < rows; iz++) {
    const lz = (iz * invNz * 2 - 1) * halfD;
    const nz = lz / halfD;
    for (let ix = 0; ix < cols; ix++) {
      const lx = (ix * invNx * 2 - 1) * halfW;
      const nx = lx / halfW;
      let h = 0;
      if (!hasStrokes) {
        const rDist = Math.hypot(nx, nz);
        if (rDist < 1) {
          const profile = Math.cos(rDist * (Math.PI / 2)) ** 1.35;
          const n =
            Math.sin(nx * 3.1415 + 0.5) * Math.cos(nz * 3.1415 + 0.3) +
            Math.sin(nx * 6.28 + nz * 4.71) * 0.35 +
            Math.cos(nx * 9.42 - nz * 7.85) * 0.15;
          const ridge = (1 - Math.abs(Math.sin(nx * 1.6 + nz * 0.9))) * 0.22;
          h = Math.max(0, height * profile * (0.85 + 0.22 * n + ridge * (1 - rDist)));
        }
      } else {
        for (let s = 0; s + 3 < strokes.length; s += 4) {
          const sx = strokes[s]!;
          const sz = strokes[s + 1]!;
          const sr = Math.max(1, strokes[s + 2]!);
          const sh = strokes[s + 3]!;
          const d = Math.hypot(lx - sx, lz - sz) / sr;
          if (d < 1) {
            const prof = Math.cos(d * (Math.PI / 2)) ** 1.35;
            const lnx = lx / sr;
            const lnz = lz / sr;
            const n =
              Math.sin(lnx * 3.1415 + 0.5) * Math.cos(lnz * 3.1415 + 0.3) +
              Math.sin(lnx * 6.28 + lnz * 4.71) * 0.35 +
              Math.cos(lnx * 9.42 - lnz * 7.85) * 0.15;
            const cand = Math.max(0, sh * prof * (0.88 + 0.16 * n));
            if (cand > h) h = cand;
          }
        }
      }
      heights[iz * cols + ix] = h;
    }
  }
  return heights;
}

/**
 * Spatial-Hash-Connector-Matching für `updatePieceConnections` (Rust-WASM wenn geladen, sonst null -> JS-Pfad).
 */
export function matchConnectorsSpatialKernel(
  connectors: Float64Array,
  distTolerance: number,
  angleTolerance: number,
  heightTolerance: number,
): Int32Array | null {
  if (wasmMod?.match_connectors_spatial) {
    return wasmMod.match_connectors_spatial(
      connectors,
      distTolerance,
      angleTolerance,
      heightTolerance,
    );
  }
  return null;
}

/**
 * 3D-Boden-Gelände-Carving & Polygon-Clipping auf dem Vertex-Gitter (Rust-WASM wenn geladen, sonst null -> JS-Pfad).
 */
export function carveGroundTerrainGridKernel(
  segsX: number,
  segsZ: number,
  width: number,
  depth: number,
  maxHeight: number,
  baseLayerElev: number,
  modelX: number,
  modelZ: number,
  cosM: number,
  sinM: number,
  transitionDist: number,
  isWater: boolean,
  rDistGrid: Float32Array,
  supportPolyOffsets: Uint32Array,
  supportPolyCoords: Float32Array,
  buildings: Float32Array,
  corridorMeta: Float32Array,
  corridorPts: Float32Array,
): Float32Array | null {
  if (wasmMod?.carve_ground_terrain_grid) {
    return wasmMod.carve_ground_terrain_grid(
      segsX,
      segsZ,
      width,
      depth,
      maxHeight,
      baseLayerElev,
      modelX,
      modelZ,
      cosM,
      sinM,
      transitionDist,
      isWater,
      rDistGrid,
      supportPolyOffsets,
      supportPolyCoords,
      buildings,
      corridorMeta,
      corridorPts,
    );
  }
  return null;
}

/**
 * Union-Find Kurzschluss-Solver für 2-Leiter-Polaritätsprüfung (Rust-WASM wenn geladen, sonst null).
 */
export function solveShortCircuitDsuKernel(
  terminalCount: number,
  edges: Uint32Array,
  railPairs: Uint32Array,
): Uint8Array | null {
  if (wasmMod?.solve_short_circuit_dsu) {
    return wasmMod.solve_short_circuit_dsu(terminalCount, edges, railPairs);
  }
  return null;
}

/**
 * 3D-Tunnelröhren-Sweep-Positionsberechnung (Rust-WASM wenn geladen, sonst null).
 */
export function buildTunnelSweepPositionsKernel(
  pathXyz: Float32Array,
  sectionXy: Float32Array,
): Float32Array | null {
  if (wasmMod?.build_tunnel_sweep_positions) {
    return wasmMod.build_tunnel_sweep_positions(pathXyz, sectionXy);
  }
  return null;
}

/**
 * Clamping, Deduplizierung & Kollinearitäts-Vereinfachung für Tisch-Ausschnittspolygone (Rust-WASM wenn geladen, sonst null).
 */
export function simplifyCutoutPolygonKernel(
  rawXy: Float64Array,
  minX: number,
  maxX: number,
  minY: number,
  maxY: number,
  dedupDist: number,
  collinearTol: number,
): Float64Array | null {
  if (wasmMod?.simplify_cutout_polygon) {
    return wasmMod.simplify_cutout_polygon(rawXy, minX, maxX, minY, maxY, dedupDist, collinearTol);
  }
  return null;
}

/** Kubischer Hermite-Flexgleis-Spline-Sampler (Rust-WASM wenn geladen, sonst null). */
export function sampleFlexSplinePointsKernel(
  chordDist: number,
  h1: number,
  h2: number,
  h3: number,
  numSamples: number,
): Float64Array | null {
  if (wasmMod?.sample_flex_spline_points) {
    return wasmMod.sample_flex_spline_points(chordDist, h1, h2, h3, numSamples);
  }
  return null;
}

/** Rampen-/Gleiswendel-Höhenprofil-Berechnung (Rust-WASM wenn geladen, sonst null). */
export function computeRampChainElevationsKernel(
  pieceLengths: Float64Array,
  isCurveLeft: Uint8Array,
  hStart: number,
  totalRise: number,
  targetLayerBaseH: number,
): Float64Array | null {
  if (wasmMod?.compute_ramp_chain_elevations) {
    return wasmMod.compute_ramp_chain_elevations(
      pieceLengths,
      isCurveLeft,
      hStart,
      totalRise,
      targetLayerBaseH,
    );
  }
  return null;
}

/** Batch-Kollisionsprüfung für Oberleitungsmasten gegen Gleis-Samples (Rust-WASM wenn geladen, sonst null). */
export function checkCatenaryMastClearancesKernel(
  trackSamples: Float32Array,
  candidates: Float32Array,
  verticalTol: number,
): Uint8Array | null {
  if (wasmMod?.check_catenary_mast_clearances) {
    return wasmMod.check_catenary_mast_clearances(trackSamples, candidates, verticalTol);
  }
  return null;
}

/** Batch-Auswertung von Gleis-Mittellinienpunkten gegen Bergvolumen (Rust-WASM wenn geladen, sonst null). */
export function evaluateMountainTrackSamplesKernel(
  ptsXye: Float32Array,
  mX: number,
  mZ: number,
  cosInv: number,
  sinInv: number,
  halfW: number,
  halfD: number,
  height: number,
): Float32Array | null {
  if (wasmMod?.evaluate_mountain_track_samples) {
    return wasmMod.evaluate_mountain_track_samples(
      ptsXye,
      mX,
      mZ,
      cosInv,
      sinInv,
      halfW,
      halfD,
      height,
    );
  }
  return null;
}
