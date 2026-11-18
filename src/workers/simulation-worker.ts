import { detectShortCircuit } from "@/core/simulation";
import type { Layout } from "@/core/types";
import type { Catalog } from "@/catalog/types";

export interface SimulationWorkerRequest {
  id: number;
  type: "DETECT_SHORT_CIRCUIT";
  layout: Layout;
  catalog: Catalog;
}

export interface SimulationWorkerResponse {
  id: number;
  type: "DETECT_SHORT_CIRCUIT";
  result: {
    hasShortCircuit: boolean;
    isTwoWireSystem: boolean;
    shortCircuitPieces: string[];
    shortCircuitPoints: any[];
  };
}

if (typeof self !== "undefined" && typeof self.postMessage === "function") {
  self.onmessage = (event: MessageEvent<SimulationWorkerRequest>) => {
    const { id, type, layout, catalog } = event.data;
    if (type === "DETECT_SHORT_CIRCUIT") {
      const res = detectShortCircuit(layout, catalog);
      const payload: SimulationWorkerResponse = {
        id,
        type,
        result: {
          hasShortCircuit: res.hasShortCircuit,
          isTwoWireSystem: res.isTwoWireSystem,
          shortCircuitPieces: Array.from(res.shortCircuitPieces),
          shortCircuitPoints: res.shortCircuitPoints,
        },
      };
      self.postMessage(payload);
    }
  };
}
