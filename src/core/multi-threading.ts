import type { Layout } from "@/core/types";
import type { Catalog } from "@/catalog/types";
import {
  detectShortCircuit,
  type ShortCircuitDetectionResult,
} from "@/core/simulation";
import type {
  SimulationWorkerRequest,
  SimulationWorkerResponse,
} from "@/workers/simulation-worker";

let workerInstance: Worker | null = null;
let reqSeq = 0;
const pendingRequests = new Map<
  number,
  (result: ShortCircuitDetectionResult) => void
>();

/**
 * Returns whether hardware concurrency / multi-threading is supported by the runtime.
 */
export function getThreadConcurrency(): number {
  if (typeof navigator !== "undefined" && navigator.hardwareConcurrency) {
    return navigator.hardwareConcurrency;
  }
  return 4;
}

/**
 * Initializes or returns the dedicated Web Worker instance for background simulation.
 */
function getWorker(): Worker | null {
  if (typeof Worker === "undefined") {
    return null;
  }
  if (!workerInstance) {
    try {
      workerInstance = new Worker(
        new URL("../workers/simulation-worker.ts", import.meta.url),
        { type: "module" }
      );
      workerInstance.onmessage = (
        e: MessageEvent<SimulationWorkerResponse>
      ) => {
        const { id, result } = e.data;
        const resolve = pendingRequests.get(id);
        if (resolve) {
          pendingRequests.delete(id);
          resolve({
            hasShortCircuit: result.hasShortCircuit,
            isTwoWireSystem: result.isTwoWireSystem,
            shortCircuitPieces: new Set(result.shortCircuitPieces),
            shortCircuitPoints: result.shortCircuitPoints,
          });
        }
      };
      workerInstance.onerror = (err) => {
        console.warn("[MultiThreading] Simulation worker error, falling back:", err);
        workerInstance = null;
      };
    } catch {
      workerInstance = null;
    }
  }
  return workerInstance;
}

/**
 * Executes short-circuit detection on a background worker thread when available,
 * avoiding UI thread blocks during complex track graph traversals.
 */
export async function detectShortCircuitAsync(
  layout: Layout,
  catalog: Catalog
): Promise<ShortCircuitDetectionResult> {
  const worker = getWorker();
  if (!worker) {
    // Fallback: synchronous execution on main thread
    return detectShortCircuit(layout, catalog);
  }

  const id = ++reqSeq;
  return new Promise<ShortCircuitDetectionResult>((resolve) => {
    pendingRequests.set(id, resolve);
    const msg: SimulationWorkerRequest = {
      id,
      type: "DETECT_SHORT_CIRCUIT",
      layout,
      catalog,
    };
    worker.postMessage(msg);
  });
}
